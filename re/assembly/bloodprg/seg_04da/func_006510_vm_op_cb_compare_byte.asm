; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006510
; seg_off: 04da:1170
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_cb_compare_byte
; label_comment: VM opcode 0xCB: lodsb tag, lodsw bx, lodsw; if tag==0xf1 compare bh vs gs:[0xaaa] and branch. Byte comparison of a script variable vs game state [0xaaa]. Companion to 0xCA || ALSO RECORDED as `vm_op_cb_global_pair_compare`: 0xCB global pair condition handler; compares packed token value to gs:0x0aaa/0x0aa8 || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xcb
; byte_count: 73
; boundary: cfg_blocks_15_terminals_4
; terminal: jmp 0x6558:3, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_006510_vm_op_cb_compare_byte.cpp
; routine_bytes_sha256: d62f591b2ec7c2dede39d54dd942b1ae1f8275bff60981115e4e2058b2b26097

006510:  AC                           lodsb    al, byte ptr [si]
006511:  8A D0                        mov      dl, al
006513:  AD                           lodsw    ax, word ptr [si]
006514:  8B D8                        mov      bx, ax
006516:  AD                           lodsw    ax, word ptr [si]
006517:  80 FA F1                     cmp      dl, 0xf1
00651A:  75 12                        jne      0x652e
00651C:  65 3A 3E AA 0A               cmp      bh, byte ptr gs:[0xaaa]
006521:  7C 32                        jl       0x6555
006523:  7F 33                        jg       0x6558
006525:  65 3A 1E A8 0A               cmp      bl, byte ptr gs:[0xaa8]
00652A:  7E 29                        jle      0x6555
00652C:  EB 2A                        jmp      0x6558
00652E:  80 FA F2                     cmp      dl, 0xf2
006531:  75 12                        jne      0x6545
006533:  65 3A 3E AA 0A               cmp      bh, byte ptr gs:[0xaaa]
006538:  7F 1B                        jg       0x6555
00653A:  7C 1C                        jl       0x6558
00653C:  65 3A 1E A8 0A               cmp      bl, byte ptr gs:[0xaa8]
006541:  7D 12                        jge      0x6555
006543:  EB 13                        jmp      0x6558
006545:  65 3A 3E AA 0A               cmp      bh, byte ptr gs:[0xaaa]
00654A:  75 09                        jne      0x6555
00654C:  65 3A 1E A8 0A               cmp      bl, byte ptr gs:[0xaa8]
006551:  75 02                        jne      0x6555
006553:  EB 03                        jmp      0x6558
006555:  E8 0A FF                     call     0x6462
006558:  C3                           ret     
