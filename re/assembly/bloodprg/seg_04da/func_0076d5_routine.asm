; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0076d5
; seg_off: 04da:2335
; group: seg_04da
; provenance: static_dispatch_table_target
; incoming: byte_parser_dispatch_74e5:byte_0x0a
; byte_count: 21
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x76d8:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 58844d9c314183089aa7995edc052136351df7f785786144c6f892233f49575a

0076D5:  BF 7A 24                     mov      di, 0x247a
0076D8:  AC                           lodsb    al, byte ptr [si]
0076D9:  0A C0                        or       al, al
0076DB:  78 07                        js       0x76e4
0076DD:  3C 20                        cmp      al, 0x20
0076DF:  72 03                        jb       0x76e4
0076E1:  AA                           stosb    byte ptr es:[di], al
0076E2:  EB F4                        jmp      0x76d8
0076E4:  4E                           dec      si
0076E5:  26 C6 05 00                  mov      byte ptr es:[di], 0
0076E9:  C3                           ret     
