; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x002bdd
; group: direct_calls
; provenance: direct_call_from_0x2514, direct_call_from_0x25d6
; byte_count: 1514
; boundary: cfg_blocks_30_terminals_7
; terminal: jmp 0x2fe1:1, jmp 0x3148:1, jmp 0x3189:2, ret:3
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 84ca972abc64d3f32329ea41ce675c13e657b44ceabc9dbc5ae8c9f61b498bc8

002BDD:  26 8B 5C 02                  mov      bx, word ptr es:[si + 2]
002BE1:  26 8B 7C 04                  mov      di, word ptr es:[si + 4]
002BE5:  26 8B 6C 06                  mov      bp, word ptr es:[si + 6]
002BE9:  8B 36 D0 0B                  mov      si, word ptr [0xbd0]
002BED:  0B F6                        or       si, si
002BEF:  0F 84 B6 FA                  je       0x26a9
002BF3:  66 26 8B 47 0A               mov      eax, dword ptr es:[bx + 0xa]
002BF8:  66 26 8B 55 0A               mov      edx, dword ptr es:[di + 0xa]
002BFD:  66 26 8B 4E 0A               mov      ecx, dword ptr es:[bp + 0xa]
002C02:  66 89 16 EC 08               mov      dword ptr [0x8ec], edx
002C07:  66 89 0E F4 08               mov      dword ptr [0x8f4], ecx
002C0C:  2B D0                        sub      dx, ax
002C0E:  0F 84 DD 03                  je       0x2fef
002C12:  2B C8                        sub      cx, ax
002C14:  0F 84 AE 05                  je       0x31c6
002C18:  66 A3 E4 08                  mov      dword ptr [0x8e4], eax
002C1C:  33 C0                        xor      ax, ax
002C1E:  89 1E 38 09                  mov      word ptr [0x938], bx
002C22:  8B DA                        mov      bx, dx
002C24:  89 3E 3A 09                  mov      word ptr [0x93a], di
002C28:  C1 E3 02                     shl      bx, 2
002C2B:  89 2E 3C 09                  mov      word ptr [0x93c], bp
002C2F:  66 8B 3F                     mov      edi, dword ptr [bx]
002C32:  8B D9                        mov      bx, cx
002C34:  C1 E3 02                     shl      bx, 2
002C37:  49                           dec      cx
002C38:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
002C3B:  66 8B 1F                     mov      ebx, dword ptr [bx]
002C3E:  66 2B D0                     sub      edx, eax
002C41:  66 2B C8                     sub      ecx, eax
002C44:  66 C1 FA 10                  sar      edx, 0x10
002C48:  66 C1 F9 10                  sar      ecx, 0x10
002C4C:  66 0F AF D7                  imul     edx, edi
002C50:  66 0F AF CB                  imul     ecx, ebx
002C54:  66 8B E9                     mov      ebp, ecx
002C57:  66 2B EA                     sub      ebp, edx
002C5A:  0F 8D 90 03                  jge      0x2fee
002C5E:  66 C1 FD 08                  sar      ebp, 8
002C62:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
002C66:  66 F7 DD                     neg      ebp
002C69:  66 89 54 1C                  mov      dword ptr [si + 0x1c], edx
002C6D:  66 D1 F9                     sar      ecx, 1
002C70:  66 D1 FA                     sar      edx, 1
002C73:  66 03 C8                     add      ecx, eax
002C76:  66 03 D0                     add      edx, eax
002C79:  66 89 4C 08                  mov      dword ptr [si + 8], ecx
002C7D:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
002C81:  66 89 3E 3E 09               mov      dword ptr [0x93e], edi
002C86:  66 89 1E 42 09               mov      dword ptr [0x942], ebx
002C8B:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002C8F:  66 26 8B 17                  mov      edx, dword ptr es:[bx]
002C93:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002C97:  66 89 16 E8 08               mov      dword ptr [0x8e8], edx
002C9C:  66 26 8B 0F                  mov      ecx, dword ptr es:[bx]
002CA0:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002CA4:  66 89 0E F8 08               mov      dword ptr [0x8f8], ecx
002CA9:  66 26 8B 1F                  mov      ebx, dword ptr es:[bx]
002CAD:  66 89 1E F0 08               mov      dword ptr [0x8f0], ebx
002CB2:  2B DA                        sub      bx, dx
002CB4:  2B CA                        sub      cx, dx
002CB6:  66 0F BF DB                  movsx    ebx, bx
002CBA:  66 0F BF C9                  movsx    ecx, cx
002CBE:  66 0F AF 1E 3E 09            imul     ebx, dword ptr [0x93e]
002CC4:  66 0F AF 0E 42 09            imul     ecx, dword ptr [0x942]
002CCA:  66 8B C3                     mov      eax, ebx
002CCD:  66 2B C1                     sub      eax, ecx
002CD0:  66 C1 F9 08                  sar      ecx, 8
002CD4:  C1 E2 08                     shl      dx, 8
002CD7:  89 4C 4A                     mov      word ptr [si + 0x4a], cx
002CDA:  D1 F9                        sar      cx, 1
002CDC:  03 D1                        add      dx, cx
002CDE:  89 54 42                     mov      word ptr [si + 0x42], dx
002CE1:  66 99                        cdq     
002CE3:  66 F7 FD                     idiv     ebp
002CE6:  89 44 52                     mov      word ptr [si + 0x52], ax
002CE9:  66 0F B7 16 EA 08            movzx    edx, word ptr [0x8ea]
002CEF:  B0 00                        mov      al, 0
002CF1:  66 0F B7 1E F2 08            movzx    ebx, word ptr [0x8f2]
002CF7:  8A E6                        mov      ah, dh
002CF9:  66 0F B7 0E FA 08            movzx    ecx, word ptr [0x8fa]
002CFF:  C1 E0 04                     shl      ax, 4
002D02:  66 2B DA                     sub      ebx, edx
002D05:  64 03 06 04 00               add      ax, word ptr fs:[4]
002D0A:  66 2B CA                     sub      ecx, edx
002D0D:  66 0F AF 1E 3E 09            imul     ebx, dword ptr [0x93e]
002D13:  89 44 56                     mov      word ptr [si + 0x56], ax
002D16:  66 0F AF 0E 42 09            imul     ecx, dword ptr [0x942]
002D1C:  66 8B C3                     mov      eax, ebx
002D1F:  66 2B C1                     sub      eax, ecx
002D22:  66 C1 F9 08                  sar      ecx, 8
002D26:  C1 E2 08                     shl      dx, 8
002D29:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
002D2C:  D1 F9                        sar      cx, 1
002D2E:  03 D1                        add      dx, cx
002D30:  89 54 44                     mov      word ptr [si + 0x44], dx
002D33:  66 99                        cdq     
002D35:  66 F7 FD                     idiv     ebp
002D38:  89 44 54                     mov      word ptr [si + 0x54], ax
002D3B:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002D3F:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002D44:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002D48:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002D4D:  66 2B C1                     sub      eax, ecx
002D50:  66 F7 2E 42 09               imul     dword ptr [0x942]
002D55:  66 0F AC D0 10               shrd     eax, edx, 0x10
002D5A:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
002D5E:  66 D1 F8                     sar      eax, 1
002D61:  66 03 C8                     add      ecx, eax
002D64:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
002D68:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002D6C:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002D71:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002D75:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002D7A:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002D7E:  66 26 8B 5F 0E               mov      ebx, dword ptr es:[bx + 0xe]
002D83:  66 2B C8                     sub      ecx, eax
002D86:  66 2B D8                     sub      ebx, eax
002D89:  66 0F AF 0E 3E 09            imul     ecx, dword ptr [0x93e]
002D8F:  66 0F AF 1E 42 09            imul     ebx, dword ptr [0x942]
002D95:  66 8B C1                     mov      eax, ecx
002D98:  66 2B C3                     sub      eax, ebx
002D9B:  66 99                        cdq     
002D9D:  66 F7 FD                     idiv     ebp
002DA0:  66 C1 F8 08                  sar      eax, 8
002DA4:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
002DA8:  A1 F4 08                     mov      ax, word ptr [0x8f4]
002DAB:  8B 1E EC 08                  mov      bx, word ptr [0x8ec]
002DAF:  2B D8                        sub      bx, ax
002DB1:  0F 88 E9 01                  js       0x2f9e
002DB5:  0F 84 82 03                  je       0x313b
002DB9:  0B C0                        or       ax, ax
002DBB:  0F 88 AA 00                  js       0x2e69
002DBF:  4B                           dec      bx
002DC0:  89 5C 30                     mov      word ptr [si + 0x30], bx
002DC3:  C1 E3 02                     shl      bx, 2
002DC6:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002DCA:  66 0F BF 06 F0 08            movsx    eax, word ptr [0x8f0]
002DD0:  66 0F BF 1E F8 08            movsx    ebx, word ptr [0x8f8]
002DD6:  66 0F BF 0E F2 08            movsx    ecx, word ptr [0x8f2]
002DDC:  66 0F BF 16 FA 08            movsx    edx, word ptr [0x8fa]
002DE2:  66 2B C3                     sub      eax, ebx
002DE5:  66 2B CA                     sub      ecx, edx
002DE8:  C1 E3 08                     shl      bx, 8
002DEB:  C1 E2 08                     shl      dx, 8
002DEE:  66 0F AF C5                  imul     eax, ebp
002DF2:  66 0F AF CD                  imul     ecx, ebp
002DF6:  66 C1 F8 08                  sar      eax, 8
002DFA:  66 C1 F9 08                  sar      ecx, 8
002DFE:  89 44 4E                     mov      word ptr [si + 0x4e], ax
002E01:  89 4C 50                     mov      word ptr [si + 0x50], cx
002E04:  D1 F8                        sar      ax, 1
002E06:  D1 F9                        sar      cx, 1
002E08:  03 D8                        add      bx, ax
002E0A:  03 D1                        add      dx, cx
002E0C:  89 5C 46                     mov      word ptr [si + 0x46], bx
002E0F:  89 54 48                     mov      word ptr [si + 0x48], dx
002E12:  66 0F BF 1E F6 08            movsx    ebx, word ptr [0x8f6]
002E18:  66 0F BF 06 EE 08            movsx    eax, word ptr [0x8ee]
002E1E:  66 2B C3                     sub      eax, ebx
002E21:  66 C1 E3 10                  shl      ebx, 0x10
002E25:  66 F7 ED                     imul     ebp
002E28:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
002E2C:  66 D1 F8                     sar      eax, 1
002E2F:  66 03 D8                     add      ebx, eax
002E32:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
002E36:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002E3A:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002E3F:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002E43:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002E48:  66 2B C1                     sub      eax, ecx
002E4B:  66 F7 ED                     imul     ebp
002E4E:  66 0F AC D0 10               shrd     eax, edx, 0x10
002E53:  66 89 44 3E                  mov      dword ptr [si + 0x3e], eax
002E57:  66 D1 F8                     sar      eax, 1
002E5A:  66 03 C8                     add      ecx, eax
002E5D:  66 89 4C 3A                  mov      dword ptr [si + 0x3a], ecx
002E61:  C7 44 2C 2A 2B               mov      word ptr [si + 0x2c], 0x2b2a
002E66:  E9 78 01                     jmp      0x2fe1
002E69:  4B                           dec      bx
002E6A:  66 0F B7 F8                  movzx    edi, ax
002E6E:  03 C3                        add      ax, bx
002E70:  F7 DF                        neg      di
002E72:  89 44 2E                     mov      word ptr [si + 0x2e], ax
002E75:  C1 E3 02                     shl      bx, 2
002E78:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002E7C:  66 0F BF 06 F0 08            movsx    eax, word ptr [0x8f0]
002E82:  66 0F BF 1E F8 08            movsx    ebx, word ptr [0x8f8]
002E88:  66 0F BF 0E F2 08            movsx    ecx, word ptr [0x8f2]
002E8E:  66 0F BF 16 FA 08            movsx    edx, word ptr [0x8fa]
002E94:  66 2B C3                     sub      eax, ebx
002E97:  66 2B CA                     sub      ecx, edx
002E9A:  C1 E3 08                     shl      bx, 8
002E9D:  C1 E2 08                     shl      dx, 8
002EA0:  66 0F AF C5                  imul     eax, ebp
002EA4:  66 0F AF CD                  imul     ecx, ebp
002EA8:  66 C1 F8 08                  sar      eax, 8
002EAC:  66 C1 F9 08                  sar      ecx, 8
002EB0:  89 44 4A                     mov      word ptr [si + 0x4a], ax
002EB3:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
002EB6:  0F AF C7                     imul     ax, di
002EB9:  0F AF CF                     imul     cx, di
002EBC:  03 D8                        add      bx, ax
002EBE:  03 D1                        add      dx, cx
002EC0:  89 5C 42                     mov      word ptr [si + 0x42], bx
002EC3:  89 54 44                     mov      word ptr [si + 0x44], dx
002EC6:  66 0F BF 1E F6 08            movsx    ebx, word ptr [0x8f6]
002ECC:  66 0F BF 06 EE 08            movsx    eax, word ptr [0x8ee]
002ED2:  66 2B C3                     sub      eax, ebx
002ED5:  66 C1 E3 10                  shl      ebx, 0x10
002ED9:  66 F7 ED                     imul     ebp
002EDC:  66 89 44 0C                  mov      dword ptr [si + 0xc], eax
002EE0:  66 0F AF C7                  imul     eax, edi
002EE4:  66 03 D8                     add      ebx, eax
002EE7:  66 89 5C 08                  mov      dword ptr [si + 8], ebx
002EEB:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002EEF:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002EF4:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002EF8:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002EFD:  66 2B C1                     sub      eax, ecx
002F00:  66 F7 ED                     imul     ebp
002F03:  66 0F AC D0 10               shrd     eax, edx, 0x10
002F08:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
002F0C:  66 0F AF C7                  imul     eax, edi
002F10:  66 03 C8                     add      ecx, eax
002F13:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
002F17:  66 0F B7 1E E4 08            movzx    ebx, word ptr [0x8e4]
002F1D:  F7 DB                        neg      bx
002F1F:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
002F23:  66 0F AF CB                  imul     ecx, ebx
002F27:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
002F2B:  C7 44 2C BE 2B               mov      word ptr [si + 0x2c], 0x2bbe
002F30:  E9 56 02                     jmp      0x3189
002F33:  4B                           dec      bx
002F34:  F7 DF                        neg      di
002F36:  C1 E3 02                     shl      bx, 2
002F39:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002F3D:  66 0F BF 1E EE 08            movsx    ebx, word ptr [0x8ee]
002F43:  66 0F BF 06 F6 08            movsx    eax, word ptr [0x8f6]
002F49:  66 2B C3                     sub      eax, ebx
002F4C:  66 C1 E3 10                  shl      ebx, 0x10
002F50:  66 F7 ED                     imul     ebp
002F53:  66 89 44 1C                  mov      dword ptr [si + 0x1c], eax
002F57:  66 F7 EF                     imul     edi
002F5A:  66 03 D8                     add      ebx, eax
002F5D:  66 89 5C 18                  mov      dword ptr [si + 0x18], ebx
002F61:  66 0F B7 1E E4 08            movzx    ebx, word ptr [0x8e4]
002F67:  F7 DB                        neg      bx
002F69:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
002F6C:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
002F70:  66 0F AF C3                  imul     eax, ebx
002F74:  66 01 44 08                  add      dword ptr [si + 8], eax
002F78:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
002F7C:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
002F7F:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
002F82:  66 0F AF C3                  imul     eax, ebx
002F86:  0F AF CB                     imul     cx, bx
002F89:  0F AF D3                     imul     dx, bx
002F8C:  66 01 44 20                  add      dword ptr [si + 0x20], eax
002F90:  01 4C 42                     add      word ptr [si + 0x42], cx
002F93:  01 54 44                     add      word ptr [si + 0x44], dx
002F96:  C7 44 2C BE 2B               mov      word ptr [si + 0x2c], 0x2bbe
002F9B:  E9 EB 01                     jmp      0x3189
002F9E:  66 0F B7 3E EC 08            movzx    edi, word ptr [0x8ec]
002FA4:  F7 DB                        neg      bx
002FA6:  0B FF                        or       di, di
002FA8:  78 89                        js       0x2f33
002FAA:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
002FAD:  4B                           dec      bx
002FAE:  89 5C 30                     mov      word ptr [si + 0x30], bx
002FB1:  C1 E3 02                     shl      bx, 2
002FB4:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002FB8:  66 0F BF 1E EE 08            movsx    ebx, word ptr [0x8ee]
002FBE:  66 0F BF 06 F6 08            movsx    eax, word ptr [0x8f6]
002FC4:  66 2B C3                     sub      eax, ebx
002FC7:  66 C1 E3 10                  shl      ebx, 0x10
002FCB:  66 F7 ED                     imul     ebp
002FCE:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
002FD2:  66 D1 F8                     sar      eax, 1
002FD5:  66 03 D8                     add      ebx, eax
002FD8:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
002FDC:  C7 44 2C 79 2B               mov      word ptr [si + 0x2c], 0x2b79
002FE1:  F7 06 E4 08 00 80            test     word ptr [0x8e4], 0x8000
002FE7:  0F 84 9E 01                  je       0x3189
002FEB:  E9 5A 01                     jmp      0x3148
002FEE:  C3                           ret     
002FEF:  2B C8                        sub      cx, ax
002FF1:  74 FB                        je       0x2fee
002FF3:  81 F9 F4 01                  cmp      cx, 0x1f4
002FF7:  73 F5                        jae      0x2fee
002FF9:  66 A3 E4 08                  mov      dword ptr [0x8e4], eax
002FFD:  33 C0                        xor      ax, ax
002FFF:  66 8B F2                     mov      esi, edx
003002:  66 2B F0                     sub      esi, eax
003005:  7E E7                        jle      0x2fee
003007:  66 C1 EE 0E                  shr      esi, 0xe
00300B:  81 FE D0 07                  cmp      si, 0x7d0
00300F:  73 DD                        jae      0x2fee
003011:  89 1E 38 09                  mov      word ptr [0x938], bx
003015:  89 3E 3A 09                  mov      word ptr [0x93a], di
003019:  89 2E 3C 09                  mov      word ptr [0x93c], bp
00301D:  8B D9                        mov      bx, cx
00301F:  66 8B 3C                     mov      edi, dword ptr [si]
003022:  C1 E3 02                     shl      bx, 2
003025:  49                           dec      cx
003026:  8B 36 D0 0B                  mov      si, word ptr [0xbd0]
00302A:  66 8B 2F                     mov      ebp, dword ptr [bx]
00302D:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
003030:  66 8B D9                     mov      ebx, ecx
003033:  66 2B C8                     sub      ecx, eax
003036:  66 C1 F9 10                  sar      ecx, 0x10
00303A:  66 0F AF CD                  imul     ecx, ebp
00303E:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
003042:  66 D1 F9                     sar      ecx, 1
003045:  66 03 C1                     add      eax, ecx
003048:  66 89 44 08                  mov      dword ptr [si + 8], eax
00304C:  66 2B DA                     sub      ebx, edx
00304F:  66 C1 FB 10                  sar      ebx, 0x10
003053:  66 0F AF DD                  imul     ebx, ebp
003057:  66 89 5C 1C                  mov      dword ptr [si + 0x1c], ebx
00305B:  66 D1 FB                     sar      ebx, 1
00305E:  66 03 D3                     add      edx, ebx
003061:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
003065:  8B 1E 38 09                  mov      bx, word ptr [0x938]
003069:  B0 00                        mov      al, 0
00306B:  66 26 0F B7 57 02            movzx    edx, word ptr es:[bx + 2]
003071:  8A E6                        mov      ah, dh
003073:  66 26 0F BF 0F               movsx    ecx, word ptr es:[bx]
003078:  C1 E0 04                     shl      ax, 4
00307B:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
00307F:  64 03 06 04 00               add      ax, word ptr fs:[4]
003084:  89 44 56                     mov      word ptr [si + 0x56], ax
003087:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
00308C:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
003092:  66 2B C1                     sub      eax, ecx
003095:  66 2B DA                     sub      ebx, edx
003098:  66 0F AF C7                  imul     eax, edi
00309C:  66 0F AF DF                  imul     ebx, edi
0030A0:  66 C1 F8 08                  sar      eax, 8
0030A4:  66 C1 FB 08                  sar      ebx, 8
0030A8:  89 44 52                     mov      word ptr [si + 0x52], ax
0030AB:  89 5C 54                     mov      word ptr [si + 0x54], bx
0030AE:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
0030B2:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
0030B7:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
0030BD:  66 2B C1                     sub      eax, ecx
0030C0:  66 2B DA                     sub      ebx, edx
0030C3:  66 0F AF C5                  imul     eax, ebp
0030C7:  66 0F AF DD                  imul     ebx, ebp
0030CB:  66 C1 F8 08                  sar      eax, 8
0030CF:  66 C1 FB 08                  sar      ebx, 8
0030D3:  89 44 4A                     mov      word ptr [si + 0x4a], ax
0030D6:  89 5C 4C                     mov      word ptr [si + 0x4c], bx
0030D9:  C1 E1 08                     shl      cx, 8
0030DC:  C1 E2 08                     shl      dx, 8
0030DF:  66 D1 F8                     sar      eax, 1
0030E2:  66 D1 FB                     sar      ebx, 1
0030E5:  03 C8                        add      cx, ax
0030E7:  03 D3                        add      dx, bx
0030E9:  89 4C 42                     mov      word ptr [si + 0x42], cx
0030EC:  89 54 44                     mov      word ptr [si + 0x44], dx
0030EF:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
0030F3:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
0030F8:  8B 1E 38 09                  mov      bx, word ptr [0x938]
0030FC:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
003101:  66 2B C1                     sub      eax, ecx
003104:  66 F7 ED                     imul     ebp
003107:  66 0F AC D0 10               shrd     eax, edx, 0x10
00310C:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
003110:  66 D1 F8                     sar      eax, 1
003113:  66 03 C1                     add      eax, ecx
003116:  66 89 44 20                  mov      dword ptr [si + 0x20], eax
00311A:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
00311E:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
003123:  8B 1E 38 09                  mov      bx, word ptr [0x938]
003127:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
00312C:  66 2B C1                     sub      eax, ecx
00312F:  66 F7 EF                     imul     edi
003132:  66 0F AC D0 10               shrd     eax, edx, 0x10
003137:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
00313B:  F7 06 E4 08 00 80            test     word ptr [0x8e4], 0x8000
003141:  C7 44 2C BE 2B               mov      word ptr [si + 0x2c], 0x2bbe
003146:  74 41                        je       0x3189
003148:  66 0F B7 1E E4 08            movzx    ebx, word ptr [0x8e4]
00314E:  F7 DB                        neg      bx
003150:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
003153:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
003157:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
00315B:  66 0F AF C3                  imul     eax, ebx
00315F:  66 0F AF CB                  imul     ecx, ebx
003163:  66 01 44 08                  add      dword ptr [si + 8], eax
003167:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
00316B:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
00316F:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
003172:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
003175:  66 0F AF C3                  imul     eax, ebx
003179:  0F AF CB                     imul     cx, bx
00317C:  0F AF D3                     imul     dx, bx
00317F:  66 01 44 20                  add      dword ptr [si + 0x20], eax
003183:  01 4C 42                     add      word ptr [si + 0x42], cx
003186:  01 54 44                     add      word ptr [si + 0x44], dx
003189:  8B 04                        mov      ax, word ptr [si]
00318B:  A3 D0 0B                     mov      word ptr [0xbd0], ax
00318E:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
003192:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
003196:  BB 2C 0C                     mov      bx, 0xc2c
003199:  8B 3F                        mov      di, word ptr [bx]
00319B:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
00319F:  7C 1A                        jl       0x31bb
0031A1:  75 06                        jne      0x31a9
0031A3:  66 3B 4D 0C                  cmp      ecx, dword ptr [di + 0xc]
0031A7:  7E 12                        jle      0x31bb
0031A9:  8B DF                        mov      bx, di
0031AB:  8B 3D                        mov      di, word ptr [di]
0031AD:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
0031B1:  7F F6                        jg       0x31a9
0031B3:  75 06                        jne      0x31bb
0031B5:  66 3B 4D 08                  cmp      ecx, dword ptr [di + 8]
0031B9:  7F EE                        jg       0x31a9
0031BB:  89 37                        mov      word ptr [bx], si
0031BD:  89 5C 10                     mov      word ptr [si + 0x10], bx
0031C0:  89 3C                        mov      word ptr [si], di
0031C2:  89 75 10                     mov      word ptr [di + 0x10], si
0031C5:  C3                           ret     
0031C6:  C3                           ret     
