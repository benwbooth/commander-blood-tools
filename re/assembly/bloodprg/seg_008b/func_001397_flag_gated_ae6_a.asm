; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001397
; seg_off: 008b:04e7
; group: seg_008b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: flag_gated_ae6_a
; label_comment: flag-gated render routine: test byte gs:[0xae6],1; if clear skip to 0x13bf, else es=gs and proceed. Gates on the 0xae6 enable bit
; incoming: call@0x00b602->008b:04e7
; byte_count: 45
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_008b/func_001397_flag_gated_ae6_a.cpp
; routine_bytes_sha256: aac9686cb22869e079aa6dbbc93dd6ec7cb86a9e13d1417c851bab1c6e099bf4

001397:  50                           push     ax
001398:  06                           push     es
001399:  53                           push     bx
00139A:  51                           push     cx
00139B:  65 F6 06 E6 0A 01            test     byte ptr gs:[0xae6], 1
0013A1:  74 1C                        je       0x13bf
0013A3:  8C E8                        mov      ax, gs
0013A5:  8E C0                        mov      es, ax
0013A7:  BB 72 0B                     mov      bx, 0xb72
0013AA:  26 C6 07 0D                  mov      byte ptr es:[bx], 0xd
0013AE:  26 C6 47 02 85               mov      byte ptr es:[bx + 2], 0x85
0013B3:  B8 10 15                     mov      ax, 0x1510
0013B6:  33 C9                        xor      cx, cx
0013B8:  65 8A 0E B9 01               mov      cl, byte ptr gs:[0x1b9]
0013BD:  CD 2F                        int      0x2f
0013BF:  59                           pop      cx
0013C0:  5B                           pop      bx
0013C1:  07                           pop      es
0013C2:  58                           pop      ax
0013C3:  CB                           retf    
