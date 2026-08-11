; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x002b6d
; group: direct_calls
; provenance: direct_call_from_0x24cf, direct_call_from_0x2572
; byte_count: 1514
; boundary: cfg_blocks_30_terminals_7
; terminal: jmp 0x2f71:1, jmp 0x30d8:1, jmp 0x3119:2, ret:3
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/direct_calls/func_002b6d_routine.cpp
; routine_bytes_sha256: 92d3573f9bd1b2b3d79e3a1179f00c075fe633903d28ec02be7b5e8ba3dac38d

002B6D:  26 8B 5C 02                  mov      bx, word ptr es:[si + 2]
002B71:  26 8B 7C 04                  mov      di, word ptr es:[si + 4]
002B75:  26 8B 6C 06                  mov      bp, word ptr es:[si + 6]
002B79:  8B 36 CE 0B                  mov      si, word ptr [0xbce]
002B7D:  0B F6                        or       si, si
002B7F:  0F 84 C2 FA                  je       0x2645
002B83:  66 26 8B 47 0A               mov      eax, dword ptr es:[bx + 0xa]
002B88:  66 26 8B 55 0A               mov      edx, dword ptr es:[di + 0xa]
002B8D:  66 26 8B 4E 0A               mov      ecx, dword ptr es:[bp + 0xa]
002B92:  66 89 16 EA 08               mov      dword ptr [0x8ea], edx
002B97:  66 89 0E F2 08               mov      dword ptr [0x8f2], ecx
002B9C:  2B D0                        sub      dx, ax
002B9E:  0F 84 DD 03                  je       0x2f7f
002BA2:  2B C8                        sub      cx, ax
002BA4:  0F 84 AE 05                  je       0x3156
002BA8:  66 A3 E2 08                  mov      dword ptr [0x8e2], eax
002BAC:  33 C0                        xor      ax, ax
002BAE:  89 1E 36 09                  mov      word ptr [0x936], bx
002BB2:  8B DA                        mov      bx, dx
002BB4:  89 3E 38 09                  mov      word ptr [0x938], di
002BB8:  C1 E3 02                     shl      bx, 2
002BBB:  89 2E 3A 09                  mov      word ptr [0x93a], bp
002BBF:  66 8B 3F                     mov      edi, dword ptr [bx]
002BC2:  8B D9                        mov      bx, cx
002BC4:  C1 E3 02                     shl      bx, 2
002BC7:  49                           dec      cx
002BC8:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
002BCB:  66 8B 1F                     mov      ebx, dword ptr [bx]
002BCE:  66 2B D0                     sub      edx, eax
002BD1:  66 2B C8                     sub      ecx, eax
002BD4:  66 C1 FA 10                  sar      edx, 0x10
002BD8:  66 C1 F9 10                  sar      ecx, 0x10
002BDC:  66 0F AF D7                  imul     edx, edi
002BE0:  66 0F AF CB                  imul     ecx, ebx
002BE4:  66 8B E9                     mov      ebp, ecx
002BE7:  66 2B EA                     sub      ebp, edx
002BEA:  0F 8D 90 03                  jge      0x2f7e
002BEE:  66 C1 FD 08                  sar      ebp, 8
002BF2:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
002BF6:  66 F7 DD                     neg      ebp
002BF9:  66 89 54 1C                  mov      dword ptr [si + 0x1c], edx
002BFD:  66 D1 F9                     sar      ecx, 1
002C00:  66 D1 FA                     sar      edx, 1
002C03:  66 03 C8                     add      ecx, eax
002C06:  66 03 D0                     add      edx, eax
002C09:  66 89 4C 08                  mov      dword ptr [si + 8], ecx
002C0D:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
002C11:  66 89 3E 3C 09               mov      dword ptr [0x93c], edi
002C16:  66 89 1E 40 09               mov      dword ptr [0x940], ebx
002C1B:  8B 1E 36 09                  mov      bx, word ptr [0x936]
002C1F:  66 26 8B 17                  mov      edx, dword ptr es:[bx]
002C23:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002C27:  66 89 16 E6 08               mov      dword ptr [0x8e6], edx
002C2C:  66 26 8B 0F                  mov      ecx, dword ptr es:[bx]
002C30:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002C34:  66 89 0E F6 08               mov      dword ptr [0x8f6], ecx
002C39:  66 26 8B 1F                  mov      ebx, dword ptr es:[bx]
002C3D:  66 89 1E EE 08               mov      dword ptr [0x8ee], ebx
002C42:  2B DA                        sub      bx, dx
002C44:  2B CA                        sub      cx, dx
002C46:  66 0F BF DB                  movsx    ebx, bx
002C4A:  66 0F BF C9                  movsx    ecx, cx
002C4E:  66 0F AF 1E 3C 09            imul     ebx, dword ptr [0x93c]
002C54:  66 0F AF 0E 40 09            imul     ecx, dword ptr [0x940]
002C5A:  66 8B C3                     mov      eax, ebx
002C5D:  66 2B C1                     sub      eax, ecx
002C60:  66 C1 F9 08                  sar      ecx, 8
002C64:  C1 E2 08                     shl      dx, 8
002C67:  89 4C 4A                     mov      word ptr [si + 0x4a], cx
002C6A:  D1 F9                        sar      cx, 1
002C6C:  03 D1                        add      dx, cx
002C6E:  89 54 42                     mov      word ptr [si + 0x42], dx
002C71:  66 99                        cdq     
002C73:  66 F7 FD                     idiv     ebp
002C76:  89 44 52                     mov      word ptr [si + 0x52], ax
002C79:  66 0F B7 16 E8 08            movzx    edx, word ptr [0x8e8]
002C7F:  B0 00                        mov      al, 0
002C81:  66 0F B7 1E F0 08            movzx    ebx, word ptr [0x8f0]
002C87:  8A E6                        mov      ah, dh
002C89:  66 0F B7 0E F8 08            movzx    ecx, word ptr [0x8f8]
002C8F:  C1 E0 04                     shl      ax, 4
002C92:  66 2B DA                     sub      ebx, edx
002C95:  64 03 06 04 00               add      ax, word ptr fs:[4]
002C9A:  66 2B CA                     sub      ecx, edx
002C9D:  66 0F AF 1E 3C 09            imul     ebx, dword ptr [0x93c]
002CA3:  89 44 56                     mov      word ptr [si + 0x56], ax
002CA6:  66 0F AF 0E 40 09            imul     ecx, dword ptr [0x940]
002CAC:  66 8B C3                     mov      eax, ebx
002CAF:  66 2B C1                     sub      eax, ecx
002CB2:  66 C1 F9 08                  sar      ecx, 8
002CB6:  C1 E2 08                     shl      dx, 8
002CB9:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
002CBC:  D1 F9                        sar      cx, 1
002CBE:  03 D1                        add      dx, cx
002CC0:  89 54 44                     mov      word ptr [si + 0x44], dx
002CC3:  66 99                        cdq     
002CC5:  66 F7 FD                     idiv     ebp
002CC8:  89 44 54                     mov      word ptr [si + 0x54], ax
002CCB:  8B 1E 36 09                  mov      bx, word ptr [0x936]
002CCF:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002CD4:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002CD8:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002CDD:  66 2B C1                     sub      eax, ecx
002CE0:  66 F7 2E 40 09               imul     dword ptr [0x940]
002CE5:  66 0F AC D0 10               shrd     eax, edx, 0x10
002CEA:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
002CEE:  66 D1 F8                     sar      eax, 1
002CF1:  66 03 C8                     add      ecx, eax
002CF4:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
002CF8:  8B 1E 36 09                  mov      bx, word ptr [0x936]
002CFC:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002D01:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002D05:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002D0A:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002D0E:  66 26 8B 5F 0E               mov      ebx, dword ptr es:[bx + 0xe]
002D13:  66 2B C8                     sub      ecx, eax
002D16:  66 2B D8                     sub      ebx, eax
002D19:  66 0F AF 0E 3C 09            imul     ecx, dword ptr [0x93c]
002D1F:  66 0F AF 1E 40 09            imul     ebx, dword ptr [0x940]
002D25:  66 8B C1                     mov      eax, ecx
002D28:  66 2B C3                     sub      eax, ebx
002D2B:  66 99                        cdq     
002D2D:  66 F7 FD                     idiv     ebp
002D30:  66 C1 F8 08                  sar      eax, 8
002D34:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
002D38:  A1 F2 08                     mov      ax, word ptr [0x8f2]
002D3B:  8B 1E EA 08                  mov      bx, word ptr [0x8ea]
002D3F:  2B D8                        sub      bx, ax
002D41:  0F 88 E9 01                  js       0x2f2e
002D45:  0F 84 82 03                  je       0x30cb
002D49:  0B C0                        or       ax, ax
002D4B:  0F 88 AA 00                  js       0x2df9
002D4F:  4B                           dec      bx
002D50:  89 5C 30                     mov      word ptr [si + 0x30], bx
002D53:  C1 E3 02                     shl      bx, 2
002D56:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002D5A:  66 0F BF 06 EE 08            movsx    eax, word ptr [0x8ee]
002D60:  66 0F BF 1E F6 08            movsx    ebx, word ptr [0x8f6]
002D66:  66 0F BF 0E F0 08            movsx    ecx, word ptr [0x8f0]
002D6C:  66 0F BF 16 F8 08            movsx    edx, word ptr [0x8f8]
002D72:  66 2B C3                     sub      eax, ebx
002D75:  66 2B CA                     sub      ecx, edx
002D78:  C1 E3 08                     shl      bx, 8
002D7B:  C1 E2 08                     shl      dx, 8
002D7E:  66 0F AF C5                  imul     eax, ebp
002D82:  66 0F AF CD                  imul     ecx, ebp
002D86:  66 C1 F8 08                  sar      eax, 8
002D8A:  66 C1 F9 08                  sar      ecx, 8
002D8E:  89 44 4E                     mov      word ptr [si + 0x4e], ax
002D91:  89 4C 50                     mov      word ptr [si + 0x50], cx
002D94:  D1 F8                        sar      ax, 1
002D96:  D1 F9                        sar      cx, 1
002D98:  03 D8                        add      bx, ax
002D9A:  03 D1                        add      dx, cx
002D9C:  89 5C 46                     mov      word ptr [si + 0x46], bx
002D9F:  89 54 48                     mov      word ptr [si + 0x48], dx
002DA2:  66 0F BF 1E F4 08            movsx    ebx, word ptr [0x8f4]
002DA8:  66 0F BF 06 EC 08            movsx    eax, word ptr [0x8ec]
002DAE:  66 2B C3                     sub      eax, ebx
002DB1:  66 C1 E3 10                  shl      ebx, 0x10
002DB5:  66 F7 ED                     imul     ebp
002DB8:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
002DBC:  66 D1 F8                     sar      eax, 1
002DBF:  66 03 D8                     add      ebx, eax
002DC2:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
002DC6:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002DCA:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002DCF:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002DD3:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002DD8:  66 2B C1                     sub      eax, ecx
002DDB:  66 F7 ED                     imul     ebp
002DDE:  66 0F AC D0 10               shrd     eax, edx, 0x10
002DE3:  66 89 44 3E                  mov      dword ptr [si + 0x3e], eax
002DE7:  66 D1 F8                     sar      eax, 1
002DEA:  66 03 C8                     add      ecx, eax
002DED:  66 89 4C 3A                  mov      dword ptr [si + 0x3a], ecx
002DF1:  C7 44 2C BA 2A               mov      word ptr [si + 0x2c], 0x2aba
002DF6:  E9 78 01                     jmp      0x2f71
002DF9:  4B                           dec      bx
002DFA:  66 0F B7 F8                  movzx    edi, ax
002DFE:  03 C3                        add      ax, bx
002E00:  F7 DF                        neg      di
002E02:  89 44 2E                     mov      word ptr [si + 0x2e], ax
002E05:  C1 E3 02                     shl      bx, 2
002E08:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002E0C:  66 0F BF 06 EE 08            movsx    eax, word ptr [0x8ee]
002E12:  66 0F BF 1E F6 08            movsx    ebx, word ptr [0x8f6]
002E18:  66 0F BF 0E F0 08            movsx    ecx, word ptr [0x8f0]
002E1E:  66 0F BF 16 F8 08            movsx    edx, word ptr [0x8f8]
002E24:  66 2B C3                     sub      eax, ebx
002E27:  66 2B CA                     sub      ecx, edx
002E2A:  C1 E3 08                     shl      bx, 8
002E2D:  C1 E2 08                     shl      dx, 8
002E30:  66 0F AF C5                  imul     eax, ebp
002E34:  66 0F AF CD                  imul     ecx, ebp
002E38:  66 C1 F8 08                  sar      eax, 8
002E3C:  66 C1 F9 08                  sar      ecx, 8
002E40:  89 44 4A                     mov      word ptr [si + 0x4a], ax
002E43:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
002E46:  0F AF C7                     imul     ax, di
002E49:  0F AF CF                     imul     cx, di
002E4C:  03 D8                        add      bx, ax
002E4E:  03 D1                        add      dx, cx
002E50:  89 5C 42                     mov      word ptr [si + 0x42], bx
002E53:  89 54 44                     mov      word ptr [si + 0x44], dx
002E56:  66 0F BF 1E F4 08            movsx    ebx, word ptr [0x8f4]
002E5C:  66 0F BF 06 EC 08            movsx    eax, word ptr [0x8ec]
002E62:  66 2B C3                     sub      eax, ebx
002E65:  66 C1 E3 10                  shl      ebx, 0x10
002E69:  66 F7 ED                     imul     ebp
002E6C:  66 89 44 0C                  mov      dword ptr [si + 0xc], eax
002E70:  66 0F AF C7                  imul     eax, edi
002E74:  66 03 D8                     add      ebx, eax
002E77:  66 89 5C 08                  mov      dword ptr [si + 8], ebx
002E7B:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002E7F:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002E84:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002E88:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002E8D:  66 2B C1                     sub      eax, ecx
002E90:  66 F7 ED                     imul     ebp
002E93:  66 0F AC D0 10               shrd     eax, edx, 0x10
002E98:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
002E9C:  66 0F AF C7                  imul     eax, edi
002EA0:  66 03 C8                     add      ecx, eax
002EA3:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
002EA7:  66 0F B7 1E E2 08            movzx    ebx, word ptr [0x8e2]
002EAD:  F7 DB                        neg      bx
002EAF:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
002EB3:  66 0F AF CB                  imul     ecx, ebx
002EB7:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
002EBB:  C7 44 2C 4E 2B               mov      word ptr [si + 0x2c], 0x2b4e
002EC0:  E9 56 02                     jmp      0x3119
002EC3:  4B                           dec      bx
002EC4:  F7 DF                        neg      di
002EC6:  C1 E3 02                     shl      bx, 2
002EC9:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002ECD:  66 0F BF 1E EC 08            movsx    ebx, word ptr [0x8ec]
002ED3:  66 0F BF 06 F4 08            movsx    eax, word ptr [0x8f4]
002ED9:  66 2B C3                     sub      eax, ebx
002EDC:  66 C1 E3 10                  shl      ebx, 0x10
002EE0:  66 F7 ED                     imul     ebp
002EE3:  66 89 44 1C                  mov      dword ptr [si + 0x1c], eax
002EE7:  66 F7 EF                     imul     edi
002EEA:  66 03 D8                     add      ebx, eax
002EED:  66 89 5C 18                  mov      dword ptr [si + 0x18], ebx
002EF1:  66 0F B7 1E E2 08            movzx    ebx, word ptr [0x8e2]
002EF7:  F7 DB                        neg      bx
002EF9:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
002EFC:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
002F00:  66 0F AF C3                  imul     eax, ebx
002F04:  66 01 44 08                  add      dword ptr [si + 8], eax
002F08:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
002F0C:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
002F0F:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
002F12:  66 0F AF C3                  imul     eax, ebx
002F16:  0F AF CB                     imul     cx, bx
002F19:  0F AF D3                     imul     dx, bx
002F1C:  66 01 44 20                  add      dword ptr [si + 0x20], eax
002F20:  01 4C 42                     add      word ptr [si + 0x42], cx
002F23:  01 54 44                     add      word ptr [si + 0x44], dx
002F26:  C7 44 2C 4E 2B               mov      word ptr [si + 0x2c], 0x2b4e
002F2B:  E9 EB 01                     jmp      0x3119
002F2E:  66 0F B7 3E EA 08            movzx    edi, word ptr [0x8ea]
002F34:  F7 DB                        neg      bx
002F36:  0B FF                        or       di, di
002F38:  78 89                        js       0x2ec3
002F3A:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
002F3D:  4B                           dec      bx
002F3E:  89 5C 30                     mov      word ptr [si + 0x30], bx
002F41:  C1 E3 02                     shl      bx, 2
002F44:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002F48:  66 0F BF 1E EC 08            movsx    ebx, word ptr [0x8ec]
002F4E:  66 0F BF 06 F4 08            movsx    eax, word ptr [0x8f4]
002F54:  66 2B C3                     sub      eax, ebx
002F57:  66 C1 E3 10                  shl      ebx, 0x10
002F5B:  66 F7 ED                     imul     ebp
002F5E:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
002F62:  66 D1 F8                     sar      eax, 1
002F65:  66 03 D8                     add      ebx, eax
002F68:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
002F6C:  C7 44 2C 09 2B               mov      word ptr [si + 0x2c], 0x2b09
002F71:  F7 06 E2 08 00 80            test     word ptr [0x8e2], 0x8000
002F77:  0F 84 9E 01                  je       0x3119
002F7B:  E9 5A 01                     jmp      0x30d8
002F7E:  C3                           ret     
002F7F:  2B C8                        sub      cx, ax
002F81:  74 FB                        je       0x2f7e
002F83:  81 F9 F4 01                  cmp      cx, 0x1f4
002F87:  73 F5                        jae      0x2f7e
002F89:  66 A3 E2 08                  mov      dword ptr [0x8e2], eax
002F8D:  33 C0                        xor      ax, ax
002F8F:  66 8B F2                     mov      esi, edx
002F92:  66 2B F0                     sub      esi, eax
002F95:  7E E7                        jle      0x2f7e
002F97:  66 C1 EE 0E                  shr      esi, 0xe
002F9B:  81 FE D0 07                  cmp      si, 0x7d0
002F9F:  73 DD                        jae      0x2f7e
002FA1:  89 1E 36 09                  mov      word ptr [0x936], bx
002FA5:  89 3E 38 09                  mov      word ptr [0x938], di
002FA9:  89 2E 3A 09                  mov      word ptr [0x93a], bp
002FAD:  8B D9                        mov      bx, cx
002FAF:  66 8B 3C                     mov      edi, dword ptr [si]
002FB2:  C1 E3 02                     shl      bx, 2
002FB5:  49                           dec      cx
002FB6:  8B 36 CE 0B                  mov      si, word ptr [0xbce]
002FBA:  66 8B 2F                     mov      ebp, dword ptr [bx]
002FBD:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
002FC0:  66 8B D9                     mov      ebx, ecx
002FC3:  66 2B C8                     sub      ecx, eax
002FC6:  66 C1 F9 10                  sar      ecx, 0x10
002FCA:  66 0F AF CD                  imul     ecx, ebp
002FCE:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
002FD2:  66 D1 F9                     sar      ecx, 1
002FD5:  66 03 C1                     add      eax, ecx
002FD8:  66 89 44 08                  mov      dword ptr [si + 8], eax
002FDC:  66 2B DA                     sub      ebx, edx
002FDF:  66 C1 FB 10                  sar      ebx, 0x10
002FE3:  66 0F AF DD                  imul     ebx, ebp
002FE7:  66 89 5C 1C                  mov      dword ptr [si + 0x1c], ebx
002FEB:  66 D1 FB                     sar      ebx, 1
002FEE:  66 03 D3                     add      edx, ebx
002FF1:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
002FF5:  8B 1E 36 09                  mov      bx, word ptr [0x936]
002FF9:  B0 00                        mov      al, 0
002FFB:  66 26 0F B7 57 02            movzx    edx, word ptr es:[bx + 2]
003001:  8A E6                        mov      ah, dh
003003:  66 26 0F BF 0F               movsx    ecx, word ptr es:[bx]
003008:  C1 E0 04                     shl      ax, 4
00300B:  8B 1E 38 09                  mov      bx, word ptr [0x938]
00300F:  64 03 06 04 00               add      ax, word ptr fs:[4]
003014:  89 44 56                     mov      word ptr [si + 0x56], ax
003017:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
00301C:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
003022:  66 2B C1                     sub      eax, ecx
003025:  66 2B DA                     sub      ebx, edx
003028:  66 0F AF C7                  imul     eax, edi
00302C:  66 0F AF DF                  imul     ebx, edi
003030:  66 C1 F8 08                  sar      eax, 8
003034:  66 C1 FB 08                  sar      ebx, 8
003038:  89 44 52                     mov      word ptr [si + 0x52], ax
00303B:  89 5C 54                     mov      word ptr [si + 0x54], bx
00303E:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
003042:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
003047:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
00304D:  66 2B C1                     sub      eax, ecx
003050:  66 2B DA                     sub      ebx, edx
003053:  66 0F AF C5                  imul     eax, ebp
003057:  66 0F AF DD                  imul     ebx, ebp
00305B:  66 C1 F8 08                  sar      eax, 8
00305F:  66 C1 FB 08                  sar      ebx, 8
003063:  89 44 4A                     mov      word ptr [si + 0x4a], ax
003066:  89 5C 4C                     mov      word ptr [si + 0x4c], bx
003069:  C1 E1 08                     shl      cx, 8
00306C:  C1 E2 08                     shl      dx, 8
00306F:  66 D1 F8                     sar      eax, 1
003072:  66 D1 FB                     sar      ebx, 1
003075:  03 C8                        add      cx, ax
003077:  03 D3                        add      dx, bx
003079:  89 4C 42                     mov      word ptr [si + 0x42], cx
00307C:  89 54 44                     mov      word ptr [si + 0x44], dx
00307F:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
003083:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
003088:  8B 1E 36 09                  mov      bx, word ptr [0x936]
00308C:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
003091:  66 2B C1                     sub      eax, ecx
003094:  66 F7 ED                     imul     ebp
003097:  66 0F AC D0 10               shrd     eax, edx, 0x10
00309C:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
0030A0:  66 D1 F8                     sar      eax, 1
0030A3:  66 03 C1                     add      eax, ecx
0030A6:  66 89 44 20                  mov      dword ptr [si + 0x20], eax
0030AA:  8B 1E 38 09                  mov      bx, word ptr [0x938]
0030AE:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
0030B3:  8B 1E 36 09                  mov      bx, word ptr [0x936]
0030B7:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
0030BC:  66 2B C1                     sub      eax, ecx
0030BF:  66 F7 EF                     imul     edi
0030C2:  66 0F AC D0 10               shrd     eax, edx, 0x10
0030C7:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
0030CB:  F7 06 E2 08 00 80            test     word ptr [0x8e2], 0x8000
0030D1:  C7 44 2C 4E 2B               mov      word ptr [si + 0x2c], 0x2b4e
0030D6:  74 41                        je       0x3119
0030D8:  66 0F B7 1E E2 08            movzx    ebx, word ptr [0x8e2]
0030DE:  F7 DB                        neg      bx
0030E0:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
0030E3:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
0030E7:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
0030EB:  66 0F AF C3                  imul     eax, ebx
0030EF:  66 0F AF CB                  imul     ecx, ebx
0030F3:  66 01 44 08                  add      dword ptr [si + 8], eax
0030F7:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
0030FB:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
0030FF:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
003102:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
003105:  66 0F AF C3                  imul     eax, ebx
003109:  0F AF CB                     imul     cx, bx
00310C:  0F AF D3                     imul     dx, bx
00310F:  66 01 44 20                  add      dword ptr [si + 0x20], eax
003113:  01 4C 42                     add      word ptr [si + 0x42], cx
003116:  01 54 44                     add      word ptr [si + 0x44], dx
003119:  8B 04                        mov      ax, word ptr [si]
00311B:  A3 CE 0B                     mov      word ptr [0xbce], ax
00311E:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
003122:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
003126:  BB 2A 0C                     mov      bx, 0xc2a
003129:  8B 3F                        mov      di, word ptr [bx]
00312B:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
00312F:  7C 1A                        jl       0x314b
003131:  75 06                        jne      0x3139
003133:  66 3B 4D 0C                  cmp      ecx, dword ptr [di + 0xc]
003137:  7E 12                        jle      0x314b
003139:  8B DF                        mov      bx, di
00313B:  8B 3D                        mov      di, word ptr [di]
00313D:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
003141:  7F F6                        jg       0x3139
003143:  75 06                        jne      0x314b
003145:  66 3B 4D 08                  cmp      ecx, dword ptr [di + 8]
003149:  7F EE                        jg       0x3139
00314B:  89 37                        mov      word ptr [bx], si
00314D:  89 5C 10                     mov      word ptr [si + 0x10], bx
003150:  89 3C                        mov      word ptr [si], di
003152:  89 75 10                     mov      word ptr [di + 0x10], si
003155:  C3                           ret     
003156:  C3                           ret     
