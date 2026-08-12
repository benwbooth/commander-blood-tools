; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007788
; seg_off: 04da:23e8
; group: seg_04da
; provenance: static_dispatch_table_target
; label: fs_name_area_read
; label_comment: copies script bytes 0x20..0x7f from DS:SI into FS:0x0c74, leaves the first control/high-bit byte unconsumed, NUL-terminates, sets GS:0x27e8 to 1, and restores the caller's ES
; incoming: byte_parser_dispatch_74e5:byte_0x0e
; byte_count: 33
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x7790:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d7a9b564c65a9ad53216b618864c4ee8519e6a58ae500780e3b0aefae6116fe0

007788:  06                           push     es
007789:  8C E0                        mov      ax, fs
00778B:  8E C0                        mov      es, ax
00778D:  BF 74 0C                     mov      di, 0xc74
007790:  AC                           lodsb    al, byte ptr [si]
007791:  0A C0                        or       al, al
007793:  78 07                        js       0x779c
007795:  3C 20                        cmp      al, 0x20
007797:  72 03                        jb       0x779c
007799:  AA                           stosb    byte ptr es:[di], al
00779A:  EB F4                        jmp      0x7790
00779C:  4E                           dec      si
00779D:  26 C6 05 00                  mov      byte ptr es:[di], 0
0077A1:  65 C6 06 E8 27 01            mov      byte ptr gs:[0x27e8], 1
0077A7:  07                           pop      es
0077A8:  C3                           ret     
