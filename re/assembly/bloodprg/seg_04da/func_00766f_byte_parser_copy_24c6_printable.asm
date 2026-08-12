; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00766f
; seg_off: 04da:22cf
; group: seg_04da
; provenance: static_dispatch_table_target
; label: byte_parser_copy_24c6_printable
; label_comment: Byte-parser opcode 0x10 copies bytes 0x20 through 0x7F from DS:SI to ES:0x24C6. It leaves the first control or high-bit byte unconsumed and writes a NUL terminator without advancing DI. Adjacent routines parse later line-record fields; they are outside this function boundary.
; incoming: byte_parser_dispatch_74e5:byte_0x10
; byte_count: 21
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x7672:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 563a3edc7b8e95fd2ba6c8d95374d3c6a92c93d610460c31c04a7372bc5fb205

00766F:  BF C6 24                     mov      di, 0x24c6
007672:  AC                           lodsb    al, byte ptr [si]
007673:  0A C0                        or       al, al
007675:  78 07                        js       0x767e
007677:  3C 20                        cmp      al, 0x20
007679:  72 03                        jb       0x767e
00767B:  AA                           stosb    byte ptr es:[di], al
00767C:  EB F4                        jmp      0x7672
00767E:  4E                           dec      si
00767F:  26 C6 05 00                  mov      byte ptr es:[di], 0
007683:  C3                           ret     
