; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00763e
; seg_off: 04da:229e
; group: seg_04da
; provenance: static_dispatch_table_target
; incoming: byte_parser_dispatch_74e5:byte_0x11
; byte_count: 49
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0x7641:1, ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: bb675d3e7d745fdc18a840fff4e5864670c27ebb0aba72520a4b25cd49c9a713

00763E:  BF 09 0D                     mov      di, 0xd09
007641:  AC                           lodsb    al, byte ptr [si]
007642:  0A C0                        or       al, al
007644:  78 07                        js       0x764d
007646:  3C 20                        cmp      al, 0x20
007648:  72 03                        jb       0x764d
00764A:  AA                           stosb    byte ptr es:[di], al
00764B:  EB F4                        jmp      0x7641
00764D:  4E                           dec      si
00764E:  26 C6 05 00                  mov      byte ptr es:[di], 0
007652:  65 F7 06 93 27 01 00         test     word ptr gs:[0x2793], 1
007659:  75 13                        jne      0x766e
00765B:  1E                           push     ds
00765C:  56                           push     si
00765D:  8C E8                        mov      ax, gs
00765F:  8E D8                        mov      ds, ax
007661:  BE 06 0D                     mov      si, 0xd06
007664:  B8 01 00                     mov      ax, 1
007667:  9A 55 08 1B 0B               lcall    0xb1b, 0x855
00766C:  5E                           pop      si
00766D:  1F                           pop      ds
00766E:  C3                           ret     
