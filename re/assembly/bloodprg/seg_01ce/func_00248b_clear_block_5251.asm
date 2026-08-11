; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00248b
; seg_off: 01ce:01ab
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: clear_block_5251
; label_comment: buffer clear: es=gs; cx=0x90; di=0x5251; xor eax; stosd loop -> zeroes 0x90 (144) dwords = 576 bytes at gs:0x5251 (render-state block adjacent to back-buffer ptr 0x5229)
; incoming: call@0x00b500->01ce:01ab
; byte_count: 27
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_00248b_clear_block_5251.cpp
; routine_bytes_sha256: 3fb16f13f03026bad977ca20171ea909e4a885b15a5651318d1ea398992f96c5

00248B:  06                           push     es
00248C:  57                           push     di
00248D:  66 50                        push     eax
00248F:  51                           push     cx
002490:  8C E8                        mov      ax, gs
002492:  8E C0                        mov      es, ax
002494:  B9 90 00                     mov      cx, 0x90
002497:  BF 51 52                     mov      di, 0x5251
00249A:  66 33 C0                     xor      eax, eax
00249D:  F3 66 AB                     rep stosd dword ptr es:[di], eax
0024A0:  59                           pop      cx
0024A1:  66 58                        pop      eax
0024A3:  5F                           pop      di
0024A4:  07                           pop      es
0024A5:  CB                           retf    
