; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0014ca
; seg_off: 008b:061a
; group: seg_008b
; provenance: recursive_graph
; label: presentation_state_set
; label_comment: presentation state set: if [0xb13]&2, [0xa32]=1 and [0x2793]|=4 (mark presentation active/pending). Sets the dialogue/presentation-active flags
; byte_count: 149
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x1559:1, ret:1
; direct_callees: none
; indirect_calls: 7
; cxx_source: re/borland/bloodprg/seg_008b/func_0014ca_presentation_state_set.cpp
; routine_bytes_sha256: c772c1c2d807635b141b909ee6256798bdc032089d63cd7dc7c33a893da42590

0014CA:  50                           push     ax
0014CB:  53                           push     bx
0014CC:  51                           push     cx
0014CD:  52                           push     dx
0014CE:  55                           push     bp
0014CF:  F6 06 13 0B 02               test     byte ptr [0xb13], 2
0014D4:  0F 84 81 00                  je       0x1559
0014D8:  C7 06 32 0A 01 00            mov      word ptr [0xa32], 1
0014DE:  80 0E 93 27 04               or       byte ptr [0x2793], 4
0014E3:  B8 E2 00                     mov      ax, 0xe2
0014E6:  BB 5A 00                     mov      bx, 0x5a
0014E9:  B9 50 00                     mov      cx, 0x50
0014EC:  BA 8C 00                     mov      dx, 0x8c
0014EF:  BD 28 00                     mov      bp, 0x28
0014F2:  9A DC 0C 99 02               lcall    0x299, 0xcdc
0014F7:  B0 E8                        mov      al, 0xe8
0014F9:  9A B5 0B 99 02               lcall    0x299, 0xbb5
0014FE:  BE 7B 01                     mov      si, 0x17b
001501:  83 C3 0A                     add      bx, 0xa
001504:  BA 58 00                     mov      dx, 0x58
001507:  9A 76 01 99 02               lcall    0x299, 0x176
00150C:  BE 89 01                     mov      si, 0x189
00150F:  83 C3 14                     add      bx, 0x14
001512:  83 C2 11                     add      dx, 0x11
001515:  9A 76 01 99 02               lcall    0x299, 0x176
00151A:  BE 8D 01                     mov      si, 0x18d
00151D:  83 C3 3C                     add      bx, 0x3c
001520:  9A 76 01 99 02               lcall    0x299, 0x176
001525:  BD 55 25                     mov      bp, 0x2555
001528:  9A B5 0A 1E 07               lcall    0x71e, 0xab5
00152D:  73 06                        jae      0x1535
00152F:  FE 0E 13 0B                  dec      byte ptr [0xb13]
001533:  EB 24                        jmp      0x1559
001535:  BD 5D 25                     mov      bp, 0x255d
001538:  9A B5 0A 1E 07               lcall    0x71e, 0xab5
00153D:  73 1A                        jae      0x1559
00153F:  C6 06 13 0B 00               mov      byte ptr [0xb13], 0
001544:  83 26 93 27 FB               and      word ptr [0x2793], 0xfffb
001549:  C7 06 32 0A 0B 00            mov      word ptr [0xa32], 0xb
00154F:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
001554:  C6 06 40 0A 00               mov      byte ptr [0xa40], 0
001559:  5D                           pop      bp
00155A:  5A                           pop      dx
00155B:  59                           pop      cx
00155C:  5B                           pop      bx
00155D:  58                           pop      ax
00155E:  C3                           ret     
