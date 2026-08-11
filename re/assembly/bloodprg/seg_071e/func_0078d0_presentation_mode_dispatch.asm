; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0078d0
; seg_off: 071e:00f0
; group: seg_071e
; provenance: recursive_graph
; label: presentation_mode_dispatch
; label_comment: presentation-mode dispatcher: bp=0x2a27; test [0x2793]&0x50 / &0x40 (presentation mode bits) -> branch to the mode-specific handler. Routes the per-frame update by presentation state
; byte_count: 93
; boundary: cfg_blocks_12_terminals_2
; terminal: jmp 0x792b:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_071e/func_0078d0_presentation_mode_dispatch.cpp
; routine_bytes_sha256: c11749b0dc63915df07f9f78ffd4ae84dd79e06a576f05b5bd72707a38c919b9

0078D0:  55                           push     bp
0078D1:  BD 27 2A                     mov      bp, 0x2a27
0078D4:  F6 06 93 27 50               test     byte ptr [0x2793], 0x50
0078D9:  74 50                        je       0x792b
0078DB:  F6 06 93 27 40               test     byte ptr [0x2793], 0x40
0078E0:  74 03                        je       0x78e5
0078E2:  83 C5 30                     add      bp, 0x30
0078E5:  A1 2A 0A                     mov      ax, word ptr [0xa2a]
0078E8:  3B 46 00                     cmp      ax, word ptr [bp]
0078EB:  7C 2C                        jl       0x7919
0078ED:  2B 46 04                     sub      ax, word ptr [bp + 4]
0078F0:  3B 46 00                     cmp      ax, word ptr [bp]
0078F3:  7F 24                        jg       0x7919
0078F5:  A1 2C 0A                     mov      ax, word ptr [0xa2c]
0078F8:  3B 46 02                     cmp      ax, word ptr [bp + 2]
0078FB:  7C 1C                        jl       0x7919
0078FD:  2B 46 06                     sub      ax, word ptr [bp + 6]
007900:  3B 46 02                     cmp      ax, word ptr [bp + 2]
007903:  7F 14                        jg       0x7919
007905:  F6 06 EA 27 01               test     byte ptr [0x27ea], 1
00790A:  75 1F                        jne      0x792b
00790C:  C6 06 EA 27 01               mov      byte ptr [0x27ea], 1
007911:  C7 06 32 0A 09 00            mov      word ptr [0xa32], 9
007917:  EB 12                        jmp      0x792b
007919:  F6 06 EA 27 01               test     byte ptr [0x27ea], 1
00791E:  74 0B                        je       0x792b
007920:  C6 06 EA 27 00               mov      byte ptr [0x27ea], 0
007925:  A1 36 0A                     mov      ax, word ptr [0xa36]
007928:  A3 32 0A                     mov      word ptr [0xa32], ax
00792B:  5D                           pop      bp
00792C:  C3                           ret     
