; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006f62
; seg_off: 04da:1bc2
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c8_record_match
; label_comment: VM opcode 0xC8 FULL: identical structure, matches records tagged 0xC8 (cmp es:[bp] vs 0xc8). Confirms each C-range opcode matches records of its own type - the typed-record model is verified across C5/C6/C8, not hypothesized || ALSO RECORDED as `vm_op_c8_state_record`: VM opcode 0xC8: object/line-record state op (state-table + 0xA1-skip prologue). C-range family member; exact operation not yet fully decoded || ALSO RECORDED as `vm_op_c8_record_entry`: 0xC8 record-entry handler; consumes operand but writes es:[record]={0xc8,0,0} when record is empty || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xc8
; byte_count: 87
; boundary: cfg_blocks_13_terminals_4
; terminal: jmp 0x6fb7:3, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: f4806637fb77e7f6cabe9d15515ecb089a55c013624846291dd0650e78c09fc6

006F62:  57                           push     di
006F63:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006F68:  32 D2                        xor      dl, dl
006F6A:  8A 04                        mov      al, byte ptr [si]
006F6C:  3C A1                        cmp      al, 0xa1
006F6E:  75 03                        jne      0x6f73
006F70:  FE C2                        inc      dl
006F72:  46                           inc      si
006F73:  AD                           lodsw    ax, word ptr [si]
006F74:  8B E8                        mov      bp, ax
006F76:  AD                           lodsw    ax, word ptr [si]
006F77:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006F7D:  74 1B                        je       0x6f9a
006F7F:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006F83:  75 0F                        jne      0x6f94
006F85:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006F89:  3D C8 00                     cmp      ax, 0xc8
006F8C:  75 06                        jne      0x6f94
006F8E:  0A D2                        or       dl, dl
006F90:  75 22                        jne      0x6fb4
006F92:  EB 23                        jmp      0x6fb7
006F94:  0A D2                        or       dl, dl
006F96:  74 1C                        je       0x6fb4
006F98:  EB 1D                        jmp      0x6fb7
006F9A:  26 8B 5E 00                  mov      bx, word ptr es:[bp]
006F9E:  0B DB                        or       bx, bx
006FA0:  75 12                        jne      0x6fb4
006FA2:  26 C7 46 00 C8 00            mov      word ptr es:[bp], 0xc8
006FA8:  26 89 5E 02                  mov      word ptr es:[bp + 2], bx
006FAC:  26 C7 46 04 00 00            mov      word ptr es:[bp + 4], 0
006FB2:  EB 03                        jmp      0x6fb7
006FB4:  E8 AB F4                     call     0x6462
006FB7:  5F                           pop      di
006FB8:  C3                           ret     
