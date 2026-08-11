; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0025a4
; seg_off: 01ce:02c4
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: string_compare
; label_comment: string compare: lodsb; cmp al,es:[di]; jne mismatch. Byte-by-byte compares two strings (name/key match)
; incoming: call@0x005493->01ce:02c4
; incoming: call@0x0054b9->01ce:02c4
; incoming: call@0x0054cc->01ce:02c4
; incoming: call@0x0054df->01ce:02c4
; incoming: call@0x0054f2->01ce:02c4
; incoming: call@0x005505->01ce:02c4
; incoming: call@0x005516->01ce:02c4
; incoming: call@0x005539->01ce:02c4
; incoming: call@0x00644c->01ce:02c4
; incoming: call@0x0070b3->01ce:02c4
; incoming: call@0x0090ac->01ce:02c4
; byte_count: 22
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x25b6:1, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_0025a4_string_compare.cpp
; routine_bytes_sha256: 1691a2639d9965aec2aef0b71864c5f83041c5eccdf86acac625d778f958bdd3

0025A4:  50                           push     ax
0025A5:  56                           push     si
0025A6:  57                           push     di
0025A7:  AC                           lodsb    al, byte ptr [si]
0025A8:  26 3A 05                     cmp      al, byte ptr es:[di]
0025AB:  75 08                        jne      0x25b5
0025AD:  47                           inc      di
0025AE:  0A C0                        or       al, al
0025B0:  75 F5                        jne      0x25a7
0025B2:  F9                           stc     
0025B3:  EB 01                        jmp      0x25b6
0025B5:  F8                           clc     
0025B6:  5F                           pop      di
0025B7:  5E                           pop      si
0025B8:  58                           pop      ax
0025B9:  CB                           retf    
