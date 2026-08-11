; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0006f1
; seg_off: 0000:00f1
; group: seg_0000
; provenance: recursive_graph
; label: mem_seg_setup_es8
; label_comment: memory segment setup: si=0; ax=es; ax+=8. Advances a segment value by 8 paragraphs (skips a segment header / MCB) during allocation
; byte_count: 53
; boundary: cfg_blocks_7_terminals_1
; terminal: ret:1
; direct_callees: 0x000726
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0000/func_0006f1_mem_seg_setup_es8.cpp
; routine_bytes_sha256: 677c17d8e691e0bfa9885696da31a9777e35d867afe9b26cd7a366b02c866a44

0006F1:  06                           push     es
0006F2:  1E                           push     ds
0006F3:  51                           push     cx
0006F4:  33 F6                        xor      si, si
0006F6:  8C C0                        mov      ax, es
0006F8:  83 C0 08                     add      ax, 8
0006FB:  8E D8                        mov      ds, ax
0006FD:  8C E8                        mov      ax, gs
0006FF:  8E C0                        mov      es, ax
000701:  33 C0                        xor      ax, ax
000703:  AC                           lodsb    al, byte ptr [si]
000704:  8B C8                        mov      cx, ax
000706:  E3 1A                        jcxz     0x722
000708:  BF F2 0A                     mov      di, 0xaf2
00070B:  AC                           lodsb    al, byte ptr [si]
00070C:  3C 20                        cmp      al, 0x20
00070E:  74 03                        je       0x713
000710:  AA                           stosb    byte ptr es:[di], al
000711:  E2 F8                        loop     0x70b
000713:  26 C6 05 00                  mov      byte ptr es:[di], 0
000717:  BF F2 0A                     mov      di, 0xaf2
00071A:  E8 09 00                     call     0x726
00071D:  E3 03                        jcxz     0x722
00071F:  49                           dec      cx
000720:  75 E9                        jne      0x70b
000722:  59                           pop      cx
000723:  1F                           pop      ds
000724:  07                           pop      es
000725:  C3                           ret     
