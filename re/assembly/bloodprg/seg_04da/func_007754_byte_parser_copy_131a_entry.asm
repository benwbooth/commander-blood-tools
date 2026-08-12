; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007754
; seg_off: 04da:23b4
; group: seg_04da
; provenance: static_dispatch_table_target
; label: byte_parser_copy_131a_entry
; label_comment: Byte-parser opcode 0x0C copies bytes 0x20..0x7F from DS:SI through the ES destination offset held at GS:0x131A, leaves the first control/high-bit byte unconsumed, and NUL-terminates. It then advances the stored destination cursor by a fixed 16 bytes and increments the GS:0x131E entry count; the copied length does not control the next slot.
; incoming: byte_parser_dispatch_74e5:byte_0x0c
; byte_count: 34
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x7759:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 4c74b7b0c5779a3a71a6883e2408d5bfec92d98a20c26bf2cee47b5ff54d0108

007754:  65 8B 3E 1A 13               mov      di, word ptr gs:[0x131a]
007759:  AC                           lodsb    al, byte ptr [si]
00775A:  0A C0                        or       al, al
00775C:  78 07                        js       0x7765
00775E:  3C 20                        cmp      al, 0x20
007760:  72 03                        jb       0x7765
007762:  AA                           stosb    byte ptr es:[di], al
007763:  EB F4                        jmp      0x7759
007765:  4E                           dec      si
007766:  26 C6 05 00                  mov      byte ptr es:[di], 0
00776A:  65 83 06 1A 13 10            add      word ptr gs:[0x131a], 0x10
007770:  65 FE 06 1E 13               inc      byte ptr gs:[0x131e]
007775:  C3                           ret     
