; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006b06
; seg_off: 04da:1766
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_b8_record_readwrite
; label_comment: Shared B8/B9/BD handler: compares or writes a two-word pair at record-base.offset + operand; set mode resolves that effective offset through the directory and clears a matching link at absolute record offset GS:0x6752 + 0x16.
; incoming: vm_opcode_handlers:opcode_0xb8
; incoming: vm_opcode_handlers:opcode_0xb9
; incoming: vm_opcode_handlers:opcode_0xbd
; byte_count: 70
; boundary: cfg_blocks_8_terminals_3
; terminal: jmp 0x6b4a:2, ret:1
; direct_callees: 0x006034, 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 249e22be2bd6eaba68be52979adb3445ffe4112aa9efa6b6685ae084caefd223

006B06:  57                           push     di
006B07:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006B0C:  AD                           lodsw    ax, word ptr [si]
006B0D:  03 F8                        add      di, ax
006B0F:  AD                           lodsw    ax, word ptr [si]
006B10:  8B D8                        mov      bx, ax
006B12:  AD                           lodsw    ax, word ptr [si]
006B13:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006B19:  74 12                        je       0x6b2d
006B1B:  26 3B 1D                     cmp      bx, word ptr es:[di]
006B1E:  75 08                        jne      0x6b28
006B20:  26 3B 45 02                  cmp      ax, word ptr es:[di + 2]
006B24:  75 02                        jne      0x6b28
006B26:  EB 22                        jmp      0x6b4a
006B28:  E8 37 F9                     call     0x6462
006B2B:  EB 1D                        jmp      0x6b4a
006B2D:  26 89 1D                     mov      word ptr es:[di], bx
006B30:  26 89 45 02                  mov      word ptr es:[di + 2], ax
006B34:  8B C7                        mov      ax, di
006B36:  E8 FB F4                     call     0x6034
006B39:  65 8B 3E 52 67               mov      di, word ptr gs:[0x6752]
006B3E:  26 3B 45 16                  cmp      ax, word ptr es:[di + 0x16]
006B42:  75 06                        jne      0x6b4a
006B44:  26 C7 45 16 00 00            mov      word ptr es:[di + 0x16], 0
006B4A:  5F                           pop      di
006B4B:  C3                           ret     
