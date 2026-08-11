; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005b38
; seg_off: 04da:0798
; group: seg_04da
; provenance: recursive_graph
; label: record_c1_ship3d_action
; label_comment: kinds 0x10/0x200 record action from the presentation scan: acts when the record field is typed 0xC1 (the ship-3D opcode) — nav/ship-3D presentation maintenance, gated on gs:0x6752/0x27DF. NOT a Sequence-cutscene dispatch || ALSO RECORDED as `mem_clear_regs`: memory/register clear helper (2 calls): es=ds; zero eax/edx/... - a buffer/state clear prologue || ALSO RECORDED as `record_type_ladder`: per-record type dispatch C1/C2/C3/C4 (was record_c1_ship3d_action; ladder confirmed) || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; byte_count: 1184
; boundary: cfg_blocks_69_terminals_14
; terminal: jmp 0x5ba1:1, jmp 0x5d33:1, jmp 0x5e09:1, jmp 0x5f15:1, jmp 0x5fcd:9, ret:1
; direct_callees: 0x005fd8, 0x005ff6, 0x006023, 0x0061a6, 0x00739b, 0x007409
; indirect_calls: 7
; routine_bytes_sha256: 2d0daac856af58f268f9364938bebdb1f47dbfa88fafc80b559c0988e67b891f

005B38:  66 50                        push     eax
005B3A:  53                           push     bx
005B3B:  51                           push     cx
005B3C:  66 52                        push     edx
005B3E:  06                           push     es
005B3F:  57                           push     di
005B40:  56                           push     si
005B41:  55                           push     bp
005B42:  8C D8                        mov      ax, ds
005B44:  8E C0                        mov      es, ax
005B46:  66 33 C0                     xor      eax, eax
005B49:  66 8B D0                     mov      edx, eax
005B4C:  57                           push     di
005B4D:  56                           push     si
005B4E:  66 8B F0                     mov      esi, eax
005B51:  66 8B F8                     mov      edi, eax
005B54:  5E                           pop      si
005B55:  5F                           pop      di
005B56:  3E 8B 46 00                  mov      ax, word ptr ds:[bp]
005B5A:  3D C1 00                     cmp      ax, 0xc1
005B5D:  0F 85 5C 01                  jne      0x5cbd
005B61:  66 33 C0                     xor      eax, eax
005B64:  65 3B 36 52 67               cmp      si, word ptr gs:[0x6752]
005B69:  75 0A                        jne      0x5b75
005B6B:  65 A0 DF 27                  mov      al, byte ptr gs:[0x27df]
005B6F:  3C 04                        cmp      al, 4
005B71:  0F 82 58 04                  jb       0x5fcd
005B75:  8B 1C                        mov      bx, word ptr [si]
005B77:  B8 11 00                     mov      ax, 0x11
005B7A:  E8 A6 04                     call     0x6023
005B7D:  66 8B D0                     mov      edx, eax
005B80:  66 50                        push     eax
005B82:  67 8B 1C 30                  mov      bx, word ptr [eax + esi]
005B86:  8B 07                        mov      ax, word ptr [bx]
005B88:  83 F8 20                     cmp      ax, 0x20
005B8B:  75 04                        jne      0x5b91
005B8D:  80 67 02 FE                  and      byte ptr [bx + 2], 0xfe
005B91:  3E 8B 7E 02                  mov      di, word ptr ds:[bp + 2]
005B95:  8B 05                        mov      ax, word ptr [di]
005B97:  3B 04                        cmp      ax, word ptr [si]
005B99:  EB 06                        jmp      0x5ba1
; -- non-contiguous block: next 0x005ba1 --
005BA1:  66 58                        pop      eax
005BA3:  67 89 3C 30                  mov      word ptr [eax + esi], di
005BA7:  3E C7 46 00 00 00            mov      word ptr ds:[bp], 0
005BAD:  8B 1C                        mov      bx, word ptr [si]
005BAF:  83 FB 10                     cmp      bx, 0x10
005BB2:  0F 84 E9 00                  je       0x5c9f
005BB6:  81 FB 00 02                  cmp      bx, 0x200
005BBA:  0F 85 0F 04                  jne      0x5fcd
005BBE:  65 F7 06 F3 24 01 00         test     word ptr gs:[0x24f3], 1
005BC5:  0F 84 D6 00                  je       0x5c9f
005BC9:  65 3B 3E 1B 25               cmp      di, word ptr gs:[0x251b]
005BCE:  74 56                        je       0x5c26
005BD0:  06                           push     es
005BD1:  57                           push     di
005BD2:  65 8E 06 26 67               mov      es, word ptr gs:[0x6726]
005BD7:  83 C7 04                     add      di, 4
005BDA:  0E                           push     cs
005BDB:  E8 2B 18                     call     0x7409
005BDE:  5F                           pop      di
005BDF:  07                           pop      es
005BE0:  0B C0                        or       ax, ax
005BE2:  0F 84 B9 00                  je       0x5c9f
005BE6:  65 F6 06 A1 0B 01            test     byte ptr gs:[0xba1], 1
005BEC:  74 38                        je       0x5c26
005BEE:  50                           push     ax
005BEF:  1E                           push     ds
005BF0:  06                           push     es
005BF1:  57                           push     di
005BF2:  8C E8                        mov      ax, gs
005BF4:  8E D8                        mov      ds, ax
005BF6:  66 FF 36 19 52               push     dword ptr [0x5219]
005BFB:  66 A1 1D 52                  mov      eax, dword ptr [0x521d]
005BFF:  66 A3 19 52                  mov      dword ptr [0x5219], eax
005C03:  66 33 C0                     xor      eax, eax
005C06:  9A 3D 07 9A 0A               lcall    0xa9a, 0x73d
005C0B:  66 8F 06 19 52               pop      dword ptr [0x5219]
005C10:  9A ED 03 1B 0B               lcall    0xb1b, 0x3ed
005C15:  BE 2D 0D                     mov      si, 0xd2d
005C18:  9A 07 06 1B 0B               lcall    0xb1b, 0x607
005C1D:  9A 03 04 1B 0B               lcall    0xb1b, 0x403
005C22:  5F                           pop      di
005C23:  07                           pop      es
005C24:  1F                           pop      ds
005C25:  58                           pop      ax
005C26:  53                           push     bx
005C27:  06                           push     es
005C28:  57                           push     di
005C29:  65 8E 06 26 67               mov      es, word ptr gs:[0x6726]
005C2E:  65 8B 3E 5E 67               mov      di, word ptr gs:[0x675e]
005C33:  26 81 3D C4 00               cmp      word ptr es:[di], 0xc4
005C38:  75 21                        jne      0x5c5b
005C3A:  26 8B 45 02                  mov      ax, word ptr es:[di + 2]
005C3E:  26 C7 05 00 00               mov      word ptr es:[di], 0
005C43:  26 C7 45 02 00 00            mov      word ptr es:[di + 2], 0
005C49:  8B F8                        mov      di, ax
005C4B:  26 8B 1D                     mov      bx, word ptr es:[di]
005C4E:  B8 13 00                     mov      ax, 0x13
005C51:  E8 CF 03                     call     0x6023
005C54:  03 F8                        add      di, ax
005C56:  33 C0                        xor      ax, ax
005C58:  AB                           stosw    word ptr es:[di], ax
005C59:  AB                           stosw    word ptr es:[di], ax
005C5A:  AB                           stosw    word ptr es:[di], ax
005C5B:  5F                           pop      di
005C5C:  07                           pop      es
005C5D:  5B                           pop      bx
005C5E:  65 89 3E 1B 25               mov      word ptr gs:[0x251b], di
005C63:  65 C7 06 F3 24 09 00         mov      word ptr gs:[0x24f3], 9
005C6A:  65 C7 06 F8 67 00 00         mov      word ptr gs:[0x67f8], 0
005C71:  65 C6 06 D7 27 00            mov      byte ptr gs:[0x27d7], 0
005C77:  65 C6 06 AA 67 00            mov      byte ptr gs:[0x67aa], 0
005C7D:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
005C83:  65 C7 06 29 25 01 00         mov      word ptr gs:[0x2529], 1
005C8A:  65 C6 06 D8 27 00            mov      byte ptr gs:[0x27d8], 0
005C90:  65 A1 A5 1F                  mov      ax, word ptr gs:[0x1fa5]
005C94:  65 A3 A7 1F                  mov      word ptr gs:[0x1fa7], ax
005C98:  65 C7 06 88 67 03 00         mov      word ptr gs:[0x6788], 3
005C9F:  87 F7                        xchg     di, si
005CA1:  E8 02 05                     call     0x61a6
005CA4:  0B C0                        or       ax, ax
005CA6:  0F 84 23 03                  je       0x5fcd
005CAA:  8B F0                        mov      si, ax
005CAC:  B8 0B 00                     mov      ax, 0xb
005CAF:  E8 71 03                     call     0x6023
005CB2:  57                           push     di
005CB3:  03 F8                        add      di, ax
005CB5:  66 AD                        lodsd    eax, dword ptr [si]
005CB7:  66 AB                        stosd    dword ptr es:[di], eax
005CB9:  5F                           pop      di
005CBA:  E9 10 03                     jmp      0x5fcd
005CBD:  3D C2 00                     cmp      ax, 0xc2
005CC0:  75 75                        jne      0x5d37
005CC2:  3E 8B 76 02                  mov      si, word ptr ds:[bp + 2]
005CC6:  8B C6                        mov      ax, si
005CC8:  E8 2B 03                     call     0x5ff6
005CCB:  0F 83 FE 02                  jae      0x5fcd
005CCF:  8B 1C                        mov      bx, word ptr [si]
005CD1:  B8 11 00                     mov      ax, 0x11
005CD4:  E8 4C 03                     call     0x6023
005CD7:  66 98                        cwde    
005CD9:  67 C7 04 30 FF FF            mov      word ptr [eax + esi], 0xffff
005CDF:  3E C7 46 00 00 00            mov      word ptr ds:[bp], 0
005CE5:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
005CEB:  75 46                        jne      0x5d33
005CED:  65 F6 06 AA 67 02            test     byte ptr gs:[0x67aa], 2
005CF3:  75 3E                        jne      0x5d33
005CF5:  83 FB 02                     cmp      bx, 2
005CF8:  75 0F                        jne      0x5d09
005CFA:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
005D00:  65 C7 06 88 67 27 00         mov      word ptr gs:[0x6788], 0x27
005D07:  EB 2A                        jmp      0x5d33
005D09:  81 FB 00 04                  cmp      bx, 0x400
005D0D:  75 24                        jne      0x5d33
005D0F:  8C D8                        mov      ax, ds
005D11:  8E C0                        mov      es, ax
005D13:  8B FE                        mov      di, si
005D15:  83 C7 04                     add      di, 4
005D18:  0E                           push     cs
005D19:  E8 ED 16                     call     0x7409
005D1C:  0B C0                        or       ax, ax
005D1E:  74 13                        je       0x5d33
005D20:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
005D26:  65 C7 06 88 67 2B 00         mov      word ptr gs:[0x6788], 0x2b
005D2D:  65 80 0E AA 67 02            or       byte ptr gs:[0x67aa], 2
005D33:  07                           pop      es
005D34:  E9 96 02                     jmp      0x5fcd
005D37:  3D C3 00                     cmp      ax, 0xc3
005D3A:  75 53                        jne      0x5d8f
005D3C:  3E 8B 5E 02                  mov      bx, word ptr ds:[bp + 2]
005D40:  65 3B 1E 4E 67               cmp      bx, word ptr gs:[0x674e]
005D45:  75 39                        jne      0x5d80
005D47:  65 89 36 5A 67               mov      word ptr gs:[0x675a], si
005D4C:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
005D52:  0F 84 77 02                  je       0x5fcd
005D56:  65 F6 06 DE 0A 01            test     byte ptr gs:[0xade], 1
005D5C:  75 06                        jne      0x5d64
005D5E:  65 80 0E 19 0B 01            or       byte ptr gs:[0xb19], 1
005D64:  65 83 3E 39 0B 00            cmp      word ptr gs:[0xb39], 0
005D6A:  0F 85 5F 02                  jne      0x5fcd
005D6E:  B8 06 00                     mov      ax, 6
005D71:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
005D76:  65 C7 06 39 0B 02 00         mov      word ptr gs:[0xb39], 2
005D7D:  E9 4D 02                     jmp      0x5fcd
005D80:  3E C7 46 00 C4 00            mov      word ptr ds:[bp], 0xc4
005D86:  3E C7 46 04 00 00            mov      word ptr ds:[bp + 4], 0
005D8C:  E9 3E 02                     jmp      0x5fcd
005D8F:  3D C4 00                     cmp      ax, 0xc4
005D92:  0F 85 8C 00                  jne      0x5e22
005D96:  3E 8B 46 04                  mov      ax, word ptr ds:[bp + 4]
005D9A:  0B C0                        or       ax, ax
005D9C:  0F 85 2D 02                  jne      0x5fcd
005DA0:  65 F6 06 B6 67 01            test     byte ptr gs:[0x67b6], 1
005DA6:  0F 85 23 02                  jne      0x5fcd
005DAA:  F7 D0                        not      ax
005DAC:  3E 89 46 04                  mov      word ptr ds:[bp + 4], ax
005DB0:  3E 8B 7E 02                  mov      di, word ptr ds:[bp + 2]
005DB4:  8B 04                        mov      ax, word ptr [si]
005DB6:  83 F8 01                     cmp      ax, 1
005DB9:  75 28                        jne      0x5de3
005DBB:  65 C7 06 5A 67 00 00         mov      word ptr gs:[0x675a], 0
005DC2:  8B 1D                        mov      bx, word ptr [di]
005DC4:  B8 08 00                     mov      ax, 8
005DC7:  E8 59 02                     call     0x6023
005DCA:  0B C0                        or       ax, ax
005DCC:  74 15                        je       0x5de3
005DCE:  67 FF 04 38                  inc      word ptr [eax + edi]
005DD2:  81 4C 02 00 80               or       word ptr [si + 2], 0x8000
005DD7:  8B DF                        mov      bx, di
005DD9:  65 89 1E 98 67               mov      word ptr gs:[0x6798], bx
005DDE:  E8 BA 15                     call     0x739b
005DE1:  EB 26                        jmp      0x5e09
005DE3:  8B 05                        mov      ax, word ptr [di]
005DE5:  83 F8 01                     cmp      ax, 1
005DE8:  75 1F                        jne      0x5e09
005DEA:  8B 1C                        mov      bx, word ptr [si]
005DEC:  B8 08 00                     mov      ax, 8
005DEF:  E8 31 02                     call     0x6023
005DF2:  0B C0                        or       ax, ax
005DF4:  74 13                        je       0x5e09
005DF6:  67 FF 04 30                  inc      word ptr [eax + esi]
005DFA:  81 4C 02 00 80               or       word ptr [si + 2], 0x8000
005DFF:  8B DE                        mov      bx, si
005E01:  65 89 1E 98 67               mov      word ptr gs:[0x6798], bx
005E06:  E8 92 15                     call     0x739b
005E09:  8B 1D                        mov      bx, word ptr [di]
005E0B:  B8 13 00                     mov      ax, 0x13
005E0E:  E8 12 02                     call     0x6023
005E11:  03 F8                        add      di, ax
005E13:  C7 05 C4 00                  mov      word ptr [di], 0xc4
005E17:  89 75 02                     mov      word ptr [di + 2], si
005E1A:  C7 45 04 FF FF               mov      word ptr [di + 4], 0xffff
005E1F:  E9 AB 01                     jmp      0x5fcd
005E22:  3D C6 00                     cmp      ax, 0xc6
005E25:  0F 85 F4 00                  jne      0x5f1d
005E29:  65 80 3E 92 27 00            cmp      byte ptr gs:[0x2792], 0
005E2F:  75 20                        jne      0x5e51
005E31:  65 80 3E 7B 2A 01            cmp      byte ptr gs:[0x2a7b], 1
005E37:  0F 85 92 01                  jne      0x5fcd
005E3B:  65 FE 06 92 27               inc      byte ptr gs:[0x2792]
005E40:  65 C6 06 8B 27 08            mov      byte ptr gs:[0x278b], 8
005E46:  B8 04 00                     mov      ax, 4
005E49:  9A 41 12 99 02               lcall    0x299, 0x1241
005E4E:  E9 7C 01                     jmp      0x5fcd
005E51:  65 80 3E 8B 27 00            cmp      byte ptr gs:[0x278b], 0
005E57:  0F 85 72 01                  jne      0x5fcd
005E5B:  65 80 3E 92 27 01            cmp      byte ptr gs:[0x2792], 1
005E61:  75 1B                        jne      0x5e7e
005E63:  65 FE 06 92 27               inc      byte ptr gs:[0x2792]
005E68:  65 C6 06 7B 2A 00            mov      byte ptr gs:[0x2a7b], 0
005E6E:  65 C6 06 8A 27 00            mov      byte ptr gs:[0x278a], 0
005E74:  65 C7 06 88 67 2C 00         mov      word ptr gs:[0x6788], 0x2c
005E7B:  E9 4F 01                     jmp      0x5fcd
005E7E:  65 F6 06 B2 1F 01            test     byte ptr gs:[0x1fb2], 1
005E84:  0F 85 45 01                  jne      0x5fcd
005E88:  65 C6 06 92 27 00            mov      byte ptr gs:[0x2792], 0
005E8E:  65 C6 06 D9 27 01            mov      byte ptr gs:[0x27d9], 1
005E94:  9A B6 14 1E 07               lcall    0x71e, 0x14b6
005E99:  65 80 26 93 27 FB            and      byte ptr gs:[0x2793], 0xfb
005E9F:  33 C0                        xor      ax, ax
005EA1:  3E 89 46 00                  mov      word ptr ds:[bp], ax
005EA5:  3E 8B 7E 02                  mov      di, word ptr ds:[bp + 2]
005EA9:  3E 89 46 02                  mov      word ptr ds:[bp + 2], ax
005EAD:  3E 89 46 04                  mov      word ptr ds:[bp + 4], ax
005EB1:  8B 1C                        mov      bx, word ptr [si]
005EB3:  B8 0E 00                     mov      ax, 0xe
005EB6:  E8 6A 01                     call     0x6023
005EB9:  66 98                        cwde    
005EBB:  8B E8                        mov      bp, ax
005EBD:  67 8B 14 30                  mov      dx, word ptr [eax + esi]
005EC1:  B8 0B 00                     mov      ax, 0xb
005EC4:  E8 5C 01                     call     0x6023
005EC7:  03 C6                        add      ax, si
005EC9:  8B C8                        mov      cx, ax
005ECB:  8B 1D                        mov      bx, word ptr [di]
005ECD:  B8 0C 00                     mov      ax, 0xc
005ED0:  E8 50 01                     call     0x6023
005ED3:  67 3B 14 38                  cmp      dx, word ptr [eax + edi]
005ED7:  75 1E                        jne      0x5ef7
005ED9:  B8 0D 00                     mov      ax, 0xd
005EDC:  E8 44 01                     call     0x6023
005EDF:  67 8B 14 38                  mov      dx, word ptr [eax + edi]
005EE3:  B8 0A 00                     mov      ax, 0xa
005EE6:  E8 3A 01                     call     0x6023
005EE9:  66 98                        cwde    
005EEB:  67 66 8B 04 38               mov      eax, dword ptr [eax + edi]
005EF0:  8B D9                        mov      bx, cx
005EF2:  66 89 07                     mov      dword ptr [bx], eax
005EF5:  EB 1E                        jmp      0x5f15
005EF7:  B8 0C 00                     mov      ax, 0xc
005EFA:  E8 26 01                     call     0x6023
005EFD:  66 98                        cwde    
005EFF:  67 8B 14 38                  mov      dx, word ptr [eax + edi]
005F03:  B8 09 00                     mov      ax, 9
005F06:  E8 1A 01                     call     0x6023
005F09:  66 98                        cwde    
005F0B:  67 66 8B 04 38               mov      eax, dword ptr [eax + edi]
005F10:  8B D9                        mov      bx, cx
005F12:  66 89 07                     mov      dword ptr [bx], eax
005F15:  3E 89 12                     mov      word ptr ds:[bp + si], dx
005F18:  8B FE                        mov      di, si
005F1A:  E9 B0 00                     jmp      0x5fcd
005F1D:  3D C9 00                     cmp      ax, 0xc9
005F20:  75 38                        jne      0x5f5a
005F22:  33 C0                        xor      ax, ax
005F24:  3E 89 46 00                  mov      word ptr ds:[bp], ax
005F28:  3E 8B 7E 02                  mov      di, word ptr ds:[bp + 2]
005F2C:  3E 89 46 02                  mov      word ptr ds:[bp + 2], ax
005F30:  3E 89 46 04                  mov      word ptr ds:[bp + 4], ax
005F34:  8B 1D                        mov      bx, word ptr [di]
005F36:  B8 13 00                     mov      ax, 0x13
005F39:  E8 E7 00                     call     0x6023
005F3C:  03 F8                        add      di, ax
005F3E:  8B 05                        mov      ax, word ptr [di]
005F40:  3D C4 00                     cmp      ax, 0xc4
005F43:  0F 85 86 00                  jne      0x5fcd
005F47:  8B 45 02                     mov      ax, word ptr [di + 2]
005F4A:  3B C6                        cmp      ax, si
005F4C:  75 7F                        jne      0x5fcd
005F4E:  33 C0                        xor      ax, ax
005F50:  89 05                        mov      word ptr [di], ax
005F52:  89 45 02                     mov      word ptr [di + 2], ax
005F55:  89 45 04                     mov      word ptr [di + 4], ax
005F58:  EB 73                        jmp      0x5fcd
005F5A:  3D CD 00                     cmp      ax, 0xcd
005F5D:  75 6E                        jne      0x5fcd
005F5F:  3E 8B 7E 02                  mov      di, word ptr ds:[bp + 2]
005F63:  8B C7                        mov      ax, di
005F65:  E8 70 00                     call     0x5fd8
005F68:  8B 1D                        mov      bx, word ptr [di]
005F6A:  B8 11 00                     mov      ax, 0x11
005F6D:  E8 B3 00                     call     0x6023
005F70:  66 98                        cwde    
005F72:  66 8B D8                     mov      ebx, eax
005F75:  3E 8B 46 04                  mov      ax, word ptr ds:[bp + 4]
005F79:  67 89 04 3B                  mov      word ptr [ebx + edi], ax
005F7D:  65 A1 92 67                  mov      ax, word ptr gs:[0x6792]
005F81:  3E 89 46 00                  mov      word ptr ds:[bp], ax
005F85:  65 A1 94 67                  mov      ax, word ptr gs:[0x6794]
005F89:  3E 89 46 02                  mov      word ptr ds:[bp + 2], ax
005F8D:  3E C7 46 04 00 00            mov      word ptr ds:[bp + 4], 0
005F93:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
005F99:  75 32                        jne      0x5fcd
005F9B:  65 F6 06 AA 67 02            test     byte ptr gs:[0x67aa], 2
005FA1:  75 2A                        jne      0x5fcd
005FA3:  81 3D 00 04                  cmp      word ptr [di], 0x400
005FA7:  75 24                        jne      0x5fcd
005FA9:  06                           push     es
005FAA:  8C D8                        mov      ax, ds
005FAC:  8E C0                        mov      es, ax
005FAE:  83 C7 04                     add      di, 4
005FB1:  0E                           push     cs
005FB2:  E8 54 14                     call     0x7409
005FB5:  07                           pop      es
005FB6:  0B C0                        or       ax, ax
005FB8:  74 13                        je       0x5fcd
005FBA:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
005FC0:  65 C7 06 88 67 2B 00         mov      word ptr gs:[0x6788], 0x2b
005FC7:  65 80 0E AA 67 02            or       byte ptr gs:[0x67aa], 2
005FCD:  5D                           pop      bp
005FCE:  5E                           pop      si
005FCF:  5F                           pop      di
005FD0:  07                           pop      es
005FD1:  66 5A                        pop      edx
005FD3:  59                           pop      cx
005FD4:  5B                           pop      bx
005FD5:  66 58                        pop      eax
005FD7:  C3                           ret     
