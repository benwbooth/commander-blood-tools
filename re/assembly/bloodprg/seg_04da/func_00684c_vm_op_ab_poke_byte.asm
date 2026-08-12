; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00684c
; seg_off: 04da:14ac
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_ab_poke_byte
; label_comment: VM opcode 0xAB: consume a byte value and inline near pointer, write the value through that pointer in DS, and advance SI by three bytes total
; incoming: vm_opcode_handlers:opcode_0xab
; byte_count: 9
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 285ee9c410b37716bc1a698993a481a2df8711a6b7c6f3a22b17a33bcd387270

00684C:  AC                           lodsb    al, byte ptr [si]
00684D:  8B 1C                        mov      bx, word ptr [si]
00684F:  88 07                        mov      byte ptr [bx], al
006851:  83 C6 02                     add      si, 2
006854:  C3                           ret     
