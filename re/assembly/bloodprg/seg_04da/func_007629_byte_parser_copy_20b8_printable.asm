; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007629
; seg_off: 04da:2289
; group: seg_04da
; provenance: static_dispatch_table_target
; label: byte_parser_copy_20b8_printable
; label_comment: Byte-parser opcode 0x06 copies bytes 0x20 through 0x7F from DS:SI to ES:0x20B8. It leaves the first control or high-bit byte unconsumed and writes a NUL terminator without advancing DI.
; incoming: byte_parser_dispatch_74e5:byte_0x06
; byte_count: 21
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x762c:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d547773420a285225c387c7ef9779eac0ee7da9fcd416fd6222a4cd1d98bcd73

007629:  BF B8 20                     mov      di, 0x20b8
00762C:  AC                           lodsb    al, byte ptr [si]
00762D:  0A C0                        or       al, al
00762F:  78 07                        js       0x7638
007631:  3C 20                        cmp      al, 0x20
007633:  72 03                        jb       0x7638
007635:  AA                           stosb    byte ptr es:[di], al
007636:  EB F4                        jmp      0x762c
007638:  4E                           dec      si
007639:  26 C6 05 00                  mov      byte ptr es:[di], 0
00763D:  C3                           ret     
