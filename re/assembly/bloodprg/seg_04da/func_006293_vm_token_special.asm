; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006293
; seg_off: 04da:0ef3
; group: seg_04da
; provenance: recursive_graph
; label: vm_token_special
; label_comment: called for length-0 opcodes (A8 AC CC D3) from token_advance
; byte_count: 16
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x6293:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_006293_vm_token_special.cpp
; routine_bytes_sha256: a3ecf862dea3865807f3c95f91722c5c5e055832af9206d61ca8117b16f98d44

006293:  3B 04                        cmp      ax, word ptr [si]
006295:  74 03                        je       0x629a
006297:  46                           inc      si
006298:  EB F9                        jmp      0x6293
00629A:  83 C6 02                     add      si, 2
00629D:  3A 04                        cmp      al, byte ptr [si]
00629F:  75 01                        jne      0x62a2
0062A1:  46                           inc      si
0062A2:  C3                           ret     
