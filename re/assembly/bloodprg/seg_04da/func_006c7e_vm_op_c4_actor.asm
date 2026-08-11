; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006c7e
; seg_off: 04da:18de
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c4_actor
; label_comment: 0xC4 actor/record handler; consumes record+related u16 operands and writes es:[record]={0xc4,related,0}
; incoming: vm_opcode_handlers:opcode_0xc4
; byte_count: 154
; boundary: cfg_blocks_19_terminals_4
; terminal: jmp 0x6d16:3, ret:1
; direct_callees: 0x006023, 0x006034, 0x006462
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_006c7e_vm_op_c4_actor.cpp
; routine_bytes_sha256: d4c42c49b43f974efd105de17e3b0e5eee8564f853979d9382e12c0690effc83

006C7E:  57                           push     di
006C7F:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006C84:  32 D2                        xor      dl, dl
006C86:  8A 04                        mov      al, byte ptr [si]
006C88:  3C A1                        cmp      al, 0xa1
006C8A:  75 03                        jne      0x6c8f
006C8C:  FE C2                        inc      dl
006C8E:  46                           inc      si
006C8F:  AD                           lodsw    ax, word ptr [si]
006C90:  8B E8                        mov      bp, ax
006C92:  E8 9F F3                     call     0x6034
006C95:  8B F8                        mov      di, ax
006C97:  AD                           lodsw    ax, word ptr [si]
006C98:  26 8B 4E 00                  mov      cx, word ptr es:[bp]
006C9C:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006CA2:  74 1F                        je       0x6cc3
006CA4:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006CA9:  74 12                        je       0x6cbd
006CAB:  81 F9 C4 00                  cmp      cx, 0xc4
006CAF:  75 0C                        jne      0x6cbd
006CB1:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006CB5:  75 06                        jne      0x6cbd
006CB7:  0A D2                        or       dl, dl
006CB9:  75 58                        jne      0x6d13
006CBB:  EB 59                        jmp      0x6d16
006CBD:  0A D2                        or       dl, dl
006CBF:  74 52                        je       0x6d13
006CC1:  EB 53                        jmp      0x6d16
006CC3:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006CC8:  74 49                        je       0x6d13
006CCA:  8B D8                        mov      bx, ax
006CCC:  26 F6 47 02 01               test     byte ptr es:[bx + 2], 1
006CD1:  74 40                        je       0x6d13
006CD3:  26 8B 05                     mov      ax, word ptr es:[di]
006CD6:  83 F8 01                     cmp      ax, 1
006CD9:  74 26                        je       0x6d01
006CDB:  26 8B 07                     mov      ax, word ptr es:[bx]
006CDE:  83 F8 01                     cmp      ax, 1
006CE1:  74 1E                        je       0x6d01
006CE3:  81 F9 C4 00                  cmp      cx, 0xc4
006CE7:  74 2A                        je       0x6d13
006CE9:  8B D3                        mov      dx, bx
006CEB:  8B D8                        mov      bx, ax
006CED:  B8 13 00                     mov      ax, 0x13
006CF0:  E8 30 F3                     call     0x6023
006CF3:  8B DA                        mov      bx, dx
006CF5:  03 C3                        add      ax, bx
006CF7:  8B F8                        mov      di, ax
006CF9:  26 8B 05                     mov      ax, word ptr es:[di]
006CFC:  3D C4 00                     cmp      ax, 0xc4
006CFF:  74 12                        je       0x6d13
006D01:  26 C7 46 00 C4 00            mov      word ptr es:[bp], 0xc4
006D07:  26 89 5E 02                  mov      word ptr es:[bp + 2], bx
006D0B:  26 C7 46 04 00 00            mov      word ptr es:[bp + 4], 0
006D11:  EB 03                        jmp      0x6d16
006D13:  E8 4C F7                     call     0x6462
006D16:  5F                           pop      di
006D17:  C3                           ret     
