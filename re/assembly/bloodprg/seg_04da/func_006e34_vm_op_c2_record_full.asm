; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006e34
; seg_off: 04da:1a94
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c2_record_full
; label_comment: VM opcode 0xC2 full record handler: query optionally inverts an active-owner exact {C2, related} match; set mode silently returns on inactive owner, a related record without flag 0x20, or a full special-slot table, otherwise stores 0xffff through selector 0x11 and may request presentation line 0x27 or 0x2b
; natural_c: re/source/bloodprg/candidates/seg_04da/func_006e34_vm_op_c2_record_full.c
; incoming: vm_opcode_handlers:opcode_0xc2
; byte_count: 186
; boundary: cfg_blocks_22_terminals_5
; terminal: jmp 0x6eec:4, ret:1
; direct_callees: 0x005ff6, 0x006023, 0x006034, 0x006462, 0x007409
; indirect_calls: 0
; routine_bytes_sha256: c503e25cd21603ee43676cf09f7073b0a932589c5e42993495175c884a83f96a

006E34:  57                           push     di
006E35:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006E3A:  32 D2                        xor      dl, dl
006E3C:  8A 04                        mov      al, byte ptr [si]
006E3E:  3C A1                        cmp      al, 0xa1
006E40:  75 03                        jne      0x6e45
006E42:  FE C2                        inc      dl
006E44:  46                           inc      si
006E45:  AD                           lodsw    ax, word ptr [si]
006E46:  8B E8                        mov      bp, ax
006E48:  E8 E9 F1                     call     0x6034
006E4B:  8B F8                        mov      di, ax
006E4D:  AD                           lodsw    ax, word ptr [si]
006E4E:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006E54:  74 22                        je       0x6e78
006E56:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006E5B:  74 15                        je       0x6e72
006E5D:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006E61:  75 0F                        jne      0x6e72
006E63:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006E67:  3D C2 00                     cmp      ax, 0xc2
006E6A:  75 06                        jne      0x6e72
006E6C:  0A D2                        or       dl, dl
006E6E:  75 79                        jne      0x6ee9
006E70:  EB 7A                        jmp      0x6eec
006E72:  0A D2                        or       dl, dl
006E74:  74 73                        je       0x6ee9
006E76:  EB 74                        jmp      0x6eec
006E78:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006E7D:  74 6D                        je       0x6eec
006E7F:  8B F8                        mov      di, ax
006E81:  26 F6 45 02 20               test     byte ptr es:[di + 2], 0x20
006E86:  74 64                        je       0x6eec
006E88:  E8 6B F1                     call     0x5ff6
006E8B:  73 5F                        jae      0x6eec
006E8D:  26 8B 1D                     mov      bx, word ptr es:[di]
006E90:  B8 11 00                     mov      ax, 0x11
006E93:  E8 8D F1                     call     0x6023
006E96:  66 98                        cwde    
006E98:  67 26 C7 04 38 FF FF         mov      word ptr es:[eax + edi], 0xffff
006E9F:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
006EA5:  75 45                        jne      0x6eec
006EA7:  65 F6 06 AA 67 02            test     byte ptr gs:[0x67aa], 2
006EAD:  75 3D                        jne      0x6eec
006EAF:  83 FB 02                     cmp      bx, 2
006EB2:  75 0F                        jne      0x6ec3
006EB4:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
006EBA:  65 C7 06 88 67 27 00         mov      word ptr gs:[0x6788], 0x27
006EC1:  EB 29                        jmp      0x6eec
006EC3:  81 FB 00 04                  cmp      bx, 0x400
006EC7:  75 23                        jne      0x6eec
006EC9:  83 C7 04                     add      di, 4
006ECC:  0E                           push     cs
006ECD:  E8 39 05                     call     0x7409
006ED0:  0B C0                        or       ax, ax
006ED2:  74 18                        je       0x6eec
006ED4:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
006EDA:  65 80 0E AA 67 02            or       byte ptr gs:[0x67aa], 2
006EE0:  65 C7 06 88 67 2B 00         mov      word ptr gs:[0x6788], 0x2b
006EE7:  EB 03                        jmp      0x6eec
006EE9:  E8 76 F5                     call     0x6462
006EEC:  5F                           pop      di
006EED:  C3                           ret     
