; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0064e5
; seg_off: 04da:1145
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_ca_compare_var
; label_comment: VM opcode 0xCA: lodsw tag (dl), lodsw value; if tag==0xf1 (variable-ref marker) compare value vs gs:[0xaa6] and branch (jg). Conditional comparison of a script value/variable against game state [0xaa6] || ALSO RECORDED as `vm_op_ca_global_word_compare`: 0xCA global condition handler; compares token value to gs:0x0aa6 || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xca
; byte_count: 43
; boundary: cfg_blocks_9_terminals_3
; terminal: jmp 0x650c:2, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 3243e7b5bf26ee35b97ac7e96602826281d5fc030dea3e8b1c7b39976d0c7e9d

0064E5:  AD                           lodsw    ax, word ptr [si]
0064E6:  8A D0                        mov      dl, al
0064E8:  AD                           lodsw    ax, word ptr [si]
0064E9:  80 FA F1                     cmp      dl, 0xf1
0064EC:  75 09                        jne      0x64f7
0064EE:  65 3B 06 A6 0A               cmp      ax, word ptr gs:[0xaa6]
0064F3:  7F 1A                        jg       0x650f
0064F5:  EB 15                        jmp      0x650c
0064F7:  80 FA F2                     cmp      dl, 0xf2
0064FA:  75 09                        jne      0x6505
0064FC:  65 3B 06 A6 0A               cmp      ax, word ptr gs:[0xaa6]
006501:  7C 0C                        jl       0x650f
006503:  EB 07                        jmp      0x650c
006505:  65 3B 06 A6 0A               cmp      ax, word ptr gs:[0xaa6]
00650A:  74 03                        je       0x650f
00650C:  E8 53 FF                     call     0x6462
00650F:  C3                           ret     
