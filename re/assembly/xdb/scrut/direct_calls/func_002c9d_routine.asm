; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x002c9d
; group: direct_calls
; provenance: direct_call_from_0x25d4, direct_call_from_0x2696
; byte_count: 1514
; boundary: cfg_blocks_30_terminals_7
; terminal: jmp 0x30a1:1, jmp 0x3208:1, jmp 0x3249:2, ret:3
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/scrut/direct_calls/func_002c9d_routine.cpp
; routine_bytes_sha256: bd9371a018942ec432ea695d4046a06902cac4f1e4e21a8231667d4fb5722ff0

002C9D:  26 8B 5C 02                  mov      bx, word ptr es:[si + 2]
002CA1:  26 8B 7C 04                  mov      di, word ptr es:[si + 4]
002CA5:  26 8B 6C 06                  mov      bp, word ptr es:[si + 6]
002CA9:  8B 36 D0 0B                  mov      si, word ptr [0xbd0]
002CAD:  0B F6                        or       si, si
002CAF:  0F 84 B6 FA                  je       0x2769
002CB3:  66 26 8B 47 0A               mov      eax, dword ptr es:[bx + 0xa]
002CB8:  66 26 8B 55 0A               mov      edx, dword ptr es:[di + 0xa]
002CBD:  66 26 8B 4E 0A               mov      ecx, dword ptr es:[bp + 0xa]
002CC2:  66 89 16 EC 08               mov      dword ptr [0x8ec], edx
002CC7:  66 89 0E F4 08               mov      dword ptr [0x8f4], ecx
002CCC:  2B D0                        sub      dx, ax
002CCE:  0F 84 DD 03                  je       0x30af
002CD2:  2B C8                        sub      cx, ax
002CD4:  0F 84 AE 05                  je       0x3286
002CD8:  66 A3 E4 08                  mov      dword ptr [0x8e4], eax
002CDC:  33 C0                        xor      ax, ax
002CDE:  89 1E 38 09                  mov      word ptr [0x938], bx
002CE2:  8B DA                        mov      bx, dx
002CE4:  89 3E 3A 09                  mov      word ptr [0x93a], di
002CE8:  C1 E3 02                     shl      bx, 2
002CEB:  89 2E 3C 09                  mov      word ptr [0x93c], bp
002CEF:  66 8B 3F                     mov      edi, dword ptr [bx]
002CF2:  8B D9                        mov      bx, cx
002CF4:  C1 E3 02                     shl      bx, 2
002CF7:  49                           dec      cx
002CF8:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
002CFB:  66 8B 1F                     mov      ebx, dword ptr [bx]
002CFE:  66 2B D0                     sub      edx, eax
002D01:  66 2B C8                     sub      ecx, eax
002D04:  66 C1 FA 10                  sar      edx, 0x10
002D08:  66 C1 F9 10                  sar      ecx, 0x10
002D0C:  66 0F AF D7                  imul     edx, edi
002D10:  66 0F AF CB                  imul     ecx, ebx
002D14:  66 8B E9                     mov      ebp, ecx
002D17:  66 2B EA                     sub      ebp, edx
002D1A:  0F 8D 90 03                  jge      0x30ae
002D1E:  66 C1 FD 08                  sar      ebp, 8
002D22:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
002D26:  66 F7 DD                     neg      ebp
002D29:  66 89 54 1C                  mov      dword ptr [si + 0x1c], edx
002D2D:  66 D1 F9                     sar      ecx, 1
002D30:  66 D1 FA                     sar      edx, 1
002D33:  66 03 C8                     add      ecx, eax
002D36:  66 03 D0                     add      edx, eax
002D39:  66 89 4C 08                  mov      dword ptr [si + 8], ecx
002D3D:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
002D41:  66 89 3E 3E 09               mov      dword ptr [0x93e], edi
002D46:  66 89 1E 42 09               mov      dword ptr [0x942], ebx
002D4B:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002D4F:  66 26 8B 17                  mov      edx, dword ptr es:[bx]
002D53:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002D57:  66 89 16 E8 08               mov      dword ptr [0x8e8], edx
002D5C:  66 26 8B 0F                  mov      ecx, dword ptr es:[bx]
002D60:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002D64:  66 89 0E F8 08               mov      dword ptr [0x8f8], ecx
002D69:  66 26 8B 1F                  mov      ebx, dword ptr es:[bx]
002D6D:  66 89 1E F0 08               mov      dword ptr [0x8f0], ebx
002D72:  2B DA                        sub      bx, dx
002D74:  2B CA                        sub      cx, dx
002D76:  66 0F BF DB                  movsx    ebx, bx
002D7A:  66 0F BF C9                  movsx    ecx, cx
002D7E:  66 0F AF 1E 3E 09            imul     ebx, dword ptr [0x93e]
002D84:  66 0F AF 0E 42 09            imul     ecx, dword ptr [0x942]
002D8A:  66 8B C3                     mov      eax, ebx
002D8D:  66 2B C1                     sub      eax, ecx
002D90:  66 C1 F9 08                  sar      ecx, 8
002D94:  C1 E2 08                     shl      dx, 8
002D97:  89 4C 4A                     mov      word ptr [si + 0x4a], cx
002D9A:  D1 F9                        sar      cx, 1
002D9C:  03 D1                        add      dx, cx
002D9E:  89 54 42                     mov      word ptr [si + 0x42], dx
002DA1:  66 99                        cdq     
002DA3:  66 F7 FD                     idiv     ebp
002DA6:  89 44 52                     mov      word ptr [si + 0x52], ax
002DA9:  66 0F B7 16 EA 08            movzx    edx, word ptr [0x8ea]
002DAF:  B0 00                        mov      al, 0
002DB1:  66 0F B7 1E F2 08            movzx    ebx, word ptr [0x8f2]
002DB7:  8A E6                        mov      ah, dh
002DB9:  66 0F B7 0E FA 08            movzx    ecx, word ptr [0x8fa]
002DBF:  C1 E0 04                     shl      ax, 4
002DC2:  66 2B DA                     sub      ebx, edx
002DC5:  64 03 06 04 00               add      ax, word ptr fs:[4]
002DCA:  66 2B CA                     sub      ecx, edx
002DCD:  66 0F AF 1E 3E 09            imul     ebx, dword ptr [0x93e]
002DD3:  89 44 56                     mov      word ptr [si + 0x56], ax
002DD6:  66 0F AF 0E 42 09            imul     ecx, dword ptr [0x942]
002DDC:  66 8B C3                     mov      eax, ebx
002DDF:  66 2B C1                     sub      eax, ecx
002DE2:  66 C1 F9 08                  sar      ecx, 8
002DE6:  C1 E2 08                     shl      dx, 8
002DE9:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
002DEC:  D1 F9                        sar      cx, 1
002DEE:  03 D1                        add      dx, cx
002DF0:  89 54 44                     mov      word ptr [si + 0x44], dx
002DF3:  66 99                        cdq     
002DF5:  66 F7 FD                     idiv     ebp
002DF8:  89 44 54                     mov      word ptr [si + 0x54], ax
002DFB:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002DFF:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002E04:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002E08:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002E0D:  66 2B C1                     sub      eax, ecx
002E10:  66 F7 2E 42 09               imul     dword ptr [0x942]
002E15:  66 0F AC D0 10               shrd     eax, edx, 0x10
002E1A:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
002E1E:  66 D1 F8                     sar      eax, 1
002E21:  66 03 C8                     add      ecx, eax
002E24:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
002E28:  8B 1E 38 09                  mov      bx, word ptr [0x938]
002E2C:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002E31:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002E35:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002E3A:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002E3E:  66 26 8B 5F 0E               mov      ebx, dword ptr es:[bx + 0xe]
002E43:  66 2B C8                     sub      ecx, eax
002E46:  66 2B D8                     sub      ebx, eax
002E49:  66 0F AF 0E 3E 09            imul     ecx, dword ptr [0x93e]
002E4F:  66 0F AF 1E 42 09            imul     ebx, dword ptr [0x942]
002E55:  66 8B C1                     mov      eax, ecx
002E58:  66 2B C3                     sub      eax, ebx
002E5B:  66 99                        cdq     
002E5D:  66 F7 FD                     idiv     ebp
002E60:  66 C1 F8 08                  sar      eax, 8
002E64:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
002E68:  A1 F4 08                     mov      ax, word ptr [0x8f4]
002E6B:  8B 1E EC 08                  mov      bx, word ptr [0x8ec]
002E6F:  2B D8                        sub      bx, ax
002E71:  0F 88 E9 01                  js       0x305e
002E75:  0F 84 82 03                  je       0x31fb
002E79:  0B C0                        or       ax, ax
002E7B:  0F 88 AA 00                  js       0x2f29
002E7F:  4B                           dec      bx
002E80:  89 5C 30                     mov      word ptr [si + 0x30], bx
002E83:  C1 E3 02                     shl      bx, 2
002E86:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002E8A:  66 0F BF 06 F0 08            movsx    eax, word ptr [0x8f0]
002E90:  66 0F BF 1E F8 08            movsx    ebx, word ptr [0x8f8]
002E96:  66 0F BF 0E F2 08            movsx    ecx, word ptr [0x8f2]
002E9C:  66 0F BF 16 FA 08            movsx    edx, word ptr [0x8fa]
002EA2:  66 2B C3                     sub      eax, ebx
002EA5:  66 2B CA                     sub      ecx, edx
002EA8:  C1 E3 08                     shl      bx, 8
002EAB:  C1 E2 08                     shl      dx, 8
002EAE:  66 0F AF C5                  imul     eax, ebp
002EB2:  66 0F AF CD                  imul     ecx, ebp
002EB6:  66 C1 F8 08                  sar      eax, 8
002EBA:  66 C1 F9 08                  sar      ecx, 8
002EBE:  89 44 4E                     mov      word ptr [si + 0x4e], ax
002EC1:  89 4C 50                     mov      word ptr [si + 0x50], cx
002EC4:  D1 F8                        sar      ax, 1
002EC6:  D1 F9                        sar      cx, 1
002EC8:  03 D8                        add      bx, ax
002ECA:  03 D1                        add      dx, cx
002ECC:  89 5C 46                     mov      word ptr [si + 0x46], bx
002ECF:  89 54 48                     mov      word ptr [si + 0x48], dx
002ED2:  66 0F BF 1E F6 08            movsx    ebx, word ptr [0x8f6]
002ED8:  66 0F BF 06 EE 08            movsx    eax, word ptr [0x8ee]
002EDE:  66 2B C3                     sub      eax, ebx
002EE1:  66 C1 E3 10                  shl      ebx, 0x10
002EE5:  66 F7 ED                     imul     ebp
002EE8:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
002EEC:  66 D1 F8                     sar      eax, 1
002EEF:  66 03 D8                     add      ebx, eax
002EF2:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
002EF6:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002EFA:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002EFF:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002F03:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002F08:  66 2B C1                     sub      eax, ecx
002F0B:  66 F7 ED                     imul     ebp
002F0E:  66 0F AC D0 10               shrd     eax, edx, 0x10
002F13:  66 89 44 3E                  mov      dword ptr [si + 0x3e], eax
002F17:  66 D1 F8                     sar      eax, 1
002F1A:  66 03 C8                     add      ecx, eax
002F1D:  66 89 4C 3A                  mov      dword ptr [si + 0x3a], ecx
002F21:  C7 44 2C EA 2B               mov      word ptr [si + 0x2c], 0x2bea
002F26:  E9 78 01                     jmp      0x30a1
002F29:  4B                           dec      bx
002F2A:  66 0F B7 F8                  movzx    edi, ax
002F2E:  03 C3                        add      ax, bx
002F30:  F7 DF                        neg      di
002F32:  89 44 2E                     mov      word ptr [si + 0x2e], ax
002F35:  C1 E3 02                     shl      bx, 2
002F38:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002F3C:  66 0F BF 06 F0 08            movsx    eax, word ptr [0x8f0]
002F42:  66 0F BF 1E F8 08            movsx    ebx, word ptr [0x8f8]
002F48:  66 0F BF 0E F2 08            movsx    ecx, word ptr [0x8f2]
002F4E:  66 0F BF 16 FA 08            movsx    edx, word ptr [0x8fa]
002F54:  66 2B C3                     sub      eax, ebx
002F57:  66 2B CA                     sub      ecx, edx
002F5A:  C1 E3 08                     shl      bx, 8
002F5D:  C1 E2 08                     shl      dx, 8
002F60:  66 0F AF C5                  imul     eax, ebp
002F64:  66 0F AF CD                  imul     ecx, ebp
002F68:  66 C1 F8 08                  sar      eax, 8
002F6C:  66 C1 F9 08                  sar      ecx, 8
002F70:  89 44 4A                     mov      word ptr [si + 0x4a], ax
002F73:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
002F76:  0F AF C7                     imul     ax, di
002F79:  0F AF CF                     imul     cx, di
002F7C:  03 D8                        add      bx, ax
002F7E:  03 D1                        add      dx, cx
002F80:  89 5C 42                     mov      word ptr [si + 0x42], bx
002F83:  89 54 44                     mov      word ptr [si + 0x44], dx
002F86:  66 0F BF 1E F6 08            movsx    ebx, word ptr [0x8f6]
002F8C:  66 0F BF 06 EE 08            movsx    eax, word ptr [0x8ee]
002F92:  66 2B C3                     sub      eax, ebx
002F95:  66 C1 E3 10                  shl      ebx, 0x10
002F99:  66 F7 ED                     imul     ebp
002F9C:  66 89 44 0C                  mov      dword ptr [si + 0xc], eax
002FA0:  66 0F AF C7                  imul     eax, edi
002FA4:  66 03 D8                     add      ebx, eax
002FA7:  66 89 5C 08                  mov      dword ptr [si + 8], ebx
002FAB:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
002FAF:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
002FB4:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
002FB8:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
002FBD:  66 2B C1                     sub      eax, ecx
002FC0:  66 F7 ED                     imul     ebp
002FC3:  66 0F AC D0 10               shrd     eax, edx, 0x10
002FC8:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
002FCC:  66 0F AF C7                  imul     eax, edi
002FD0:  66 03 C8                     add      ecx, eax
002FD3:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
002FD7:  66 0F B7 1E E4 08            movzx    ebx, word ptr [0x8e4]
002FDD:  F7 DB                        neg      bx
002FDF:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
002FE3:  66 0F AF CB                  imul     ecx, ebx
002FE7:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
002FEB:  C7 44 2C 7E 2C               mov      word ptr [si + 0x2c], 0x2c7e
002FF0:  E9 56 02                     jmp      0x3249
002FF3:  4B                           dec      bx
002FF4:  F7 DF                        neg      di
002FF6:  C1 E3 02                     shl      bx, 2
002FF9:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
002FFD:  66 0F BF 1E EE 08            movsx    ebx, word ptr [0x8ee]
003003:  66 0F BF 06 F6 08            movsx    eax, word ptr [0x8f6]
003009:  66 2B C3                     sub      eax, ebx
00300C:  66 C1 E3 10                  shl      ebx, 0x10
003010:  66 F7 ED                     imul     ebp
003013:  66 89 44 1C                  mov      dword ptr [si + 0x1c], eax
003017:  66 F7 EF                     imul     edi
00301A:  66 03 D8                     add      ebx, eax
00301D:  66 89 5C 18                  mov      dword ptr [si + 0x18], ebx
003021:  66 0F B7 1E E4 08            movzx    ebx, word ptr [0x8e4]
003027:  F7 DB                        neg      bx
003029:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
00302C:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
003030:  66 0F AF C3                  imul     eax, ebx
003034:  66 01 44 08                  add      dword ptr [si + 8], eax
003038:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
00303C:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
00303F:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
003042:  66 0F AF C3                  imul     eax, ebx
003046:  0F AF CB                     imul     cx, bx
003049:  0F AF D3                     imul     dx, bx
00304C:  66 01 44 20                  add      dword ptr [si + 0x20], eax
003050:  01 4C 42                     add      word ptr [si + 0x42], cx
003053:  01 54 44                     add      word ptr [si + 0x44], dx
003056:  C7 44 2C 7E 2C               mov      word ptr [si + 0x2c], 0x2c7e
00305B:  E9 EB 01                     jmp      0x3249
00305E:  66 0F B7 3E EC 08            movzx    edi, word ptr [0x8ec]
003064:  F7 DB                        neg      bx
003066:  0B FF                        or       di, di
003068:  78 89                        js       0x2ff3
00306A:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
00306D:  4B                           dec      bx
00306E:  89 5C 30                     mov      word ptr [si + 0x30], bx
003071:  C1 E3 02                     shl      bx, 2
003074:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
003078:  66 0F BF 1E EE 08            movsx    ebx, word ptr [0x8ee]
00307E:  66 0F BF 06 F6 08            movsx    eax, word ptr [0x8f6]
003084:  66 2B C3                     sub      eax, ebx
003087:  66 C1 E3 10                  shl      ebx, 0x10
00308B:  66 F7 ED                     imul     ebp
00308E:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
003092:  66 D1 F8                     sar      eax, 1
003095:  66 03 D8                     add      ebx, eax
003098:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
00309C:  C7 44 2C 39 2C               mov      word ptr [si + 0x2c], 0x2c39
0030A1:  F7 06 E4 08 00 80            test     word ptr [0x8e4], 0x8000
0030A7:  0F 84 9E 01                  je       0x3249
0030AB:  E9 5A 01                     jmp      0x3208
0030AE:  C3                           ret     
0030AF:  2B C8                        sub      cx, ax
0030B1:  74 FB                        je       0x30ae
0030B3:  81 F9 F4 01                  cmp      cx, 0x1f4
0030B7:  73 F5                        jae      0x30ae
0030B9:  66 A3 E4 08                  mov      dword ptr [0x8e4], eax
0030BD:  33 C0                        xor      ax, ax
0030BF:  66 8B F2                     mov      esi, edx
0030C2:  66 2B F0                     sub      esi, eax
0030C5:  7E E7                        jle      0x30ae
0030C7:  66 C1 EE 0E                  shr      esi, 0xe
0030CB:  81 FE D0 07                  cmp      si, 0x7d0
0030CF:  73 DD                        jae      0x30ae
0030D1:  89 1E 38 09                  mov      word ptr [0x938], bx
0030D5:  89 3E 3A 09                  mov      word ptr [0x93a], di
0030D9:  89 2E 3C 09                  mov      word ptr [0x93c], bp
0030DD:  8B D9                        mov      bx, cx
0030DF:  66 8B 3C                     mov      edi, dword ptr [si]
0030E2:  C1 E3 02                     shl      bx, 2
0030E5:  49                           dec      cx
0030E6:  8B 36 D0 0B                  mov      si, word ptr [0xbd0]
0030EA:  66 8B 2F                     mov      ebp, dword ptr [bx]
0030ED:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
0030F0:  66 8B D9                     mov      ebx, ecx
0030F3:  66 2B C8                     sub      ecx, eax
0030F6:  66 C1 F9 10                  sar      ecx, 0x10
0030FA:  66 0F AF CD                  imul     ecx, ebp
0030FE:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
003102:  66 D1 F9                     sar      ecx, 1
003105:  66 03 C1                     add      eax, ecx
003108:  66 89 44 08                  mov      dword ptr [si + 8], eax
00310C:  66 2B DA                     sub      ebx, edx
00310F:  66 C1 FB 10                  sar      ebx, 0x10
003113:  66 0F AF DD                  imul     ebx, ebp
003117:  66 89 5C 1C                  mov      dword ptr [si + 0x1c], ebx
00311B:  66 D1 FB                     sar      ebx, 1
00311E:  66 03 D3                     add      edx, ebx
003121:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
003125:  8B 1E 38 09                  mov      bx, word ptr [0x938]
003129:  B0 00                        mov      al, 0
00312B:  66 26 0F B7 57 02            movzx    edx, word ptr es:[bx + 2]
003131:  8A E6                        mov      ah, dh
003133:  66 26 0F BF 0F               movsx    ecx, word ptr es:[bx]
003138:  C1 E0 04                     shl      ax, 4
00313B:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
00313F:  64 03 06 04 00               add      ax, word ptr fs:[4]
003144:  89 44 56                     mov      word ptr [si + 0x56], ax
003147:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
00314C:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
003152:  66 2B C1                     sub      eax, ecx
003155:  66 2B DA                     sub      ebx, edx
003158:  66 0F AF C7                  imul     eax, edi
00315C:  66 0F AF DF                  imul     ebx, edi
003160:  66 C1 F8 08                  sar      eax, 8
003164:  66 C1 FB 08                  sar      ebx, 8
003168:  89 44 52                     mov      word ptr [si + 0x52], ax
00316B:  89 5C 54                     mov      word ptr [si + 0x54], bx
00316E:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
003172:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
003177:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
00317D:  66 2B C1                     sub      eax, ecx
003180:  66 2B DA                     sub      ebx, edx
003183:  66 0F AF C5                  imul     eax, ebp
003187:  66 0F AF DD                  imul     ebx, ebp
00318B:  66 C1 F8 08                  sar      eax, 8
00318F:  66 C1 FB 08                  sar      ebx, 8
003193:  89 44 4A                     mov      word ptr [si + 0x4a], ax
003196:  89 5C 4C                     mov      word ptr [si + 0x4c], bx
003199:  C1 E1 08                     shl      cx, 8
00319C:  C1 E2 08                     shl      dx, 8
00319F:  66 D1 F8                     sar      eax, 1
0031A2:  66 D1 FB                     sar      ebx, 1
0031A5:  03 C8                        add      cx, ax
0031A7:  03 D3                        add      dx, bx
0031A9:  89 4C 42                     mov      word ptr [si + 0x42], cx
0031AC:  89 54 44                     mov      word ptr [si + 0x44], dx
0031AF:  8B 1E 3C 09                  mov      bx, word ptr [0x93c]
0031B3:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
0031B8:  8B 1E 38 09                  mov      bx, word ptr [0x938]
0031BC:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
0031C1:  66 2B C1                     sub      eax, ecx
0031C4:  66 F7 ED                     imul     ebp
0031C7:  66 0F AC D0 10               shrd     eax, edx, 0x10
0031CC:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
0031D0:  66 D1 F8                     sar      eax, 1
0031D3:  66 03 C1                     add      eax, ecx
0031D6:  66 89 44 20                  mov      dword ptr [si + 0x20], eax
0031DA:  8B 1E 3A 09                  mov      bx, word ptr [0x93a]
0031DE:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
0031E3:  8B 1E 38 09                  mov      bx, word ptr [0x938]
0031E7:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
0031EC:  66 2B C1                     sub      eax, ecx
0031EF:  66 F7 EF                     imul     edi
0031F2:  66 0F AC D0 10               shrd     eax, edx, 0x10
0031F7:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
0031FB:  F7 06 E4 08 00 80            test     word ptr [0x8e4], 0x8000
003201:  C7 44 2C 7E 2C               mov      word ptr [si + 0x2c], 0x2c7e
003206:  74 41                        je       0x3249
003208:  66 0F B7 1E E4 08            movzx    ebx, word ptr [0x8e4]
00320E:  F7 DB                        neg      bx
003210:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
003213:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
003217:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
00321B:  66 0F AF C3                  imul     eax, ebx
00321F:  66 0F AF CB                  imul     ecx, ebx
003223:  66 01 44 08                  add      dword ptr [si + 8], eax
003227:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
00322B:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
00322F:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
003232:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
003235:  66 0F AF C3                  imul     eax, ebx
003239:  0F AF CB                     imul     cx, bx
00323C:  0F AF D3                     imul     dx, bx
00323F:  66 01 44 20                  add      dword ptr [si + 0x20], eax
003243:  01 4C 42                     add      word ptr [si + 0x42], cx
003246:  01 54 44                     add      word ptr [si + 0x44], dx
003249:  8B 04                        mov      ax, word ptr [si]
00324B:  A3 D0 0B                     mov      word ptr [0xbd0], ax
00324E:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
003252:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
003256:  BB 2C 0C                     mov      bx, 0xc2c
003259:  8B 3F                        mov      di, word ptr [bx]
00325B:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
00325F:  7C 1A                        jl       0x327b
003261:  75 06                        jne      0x3269
003263:  66 3B 4D 0C                  cmp      ecx, dword ptr [di + 0xc]
003267:  7E 12                        jle      0x327b
003269:  8B DF                        mov      bx, di
00326B:  8B 3D                        mov      di, word ptr [di]
00326D:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
003271:  7F F6                        jg       0x3269
003273:  75 06                        jne      0x327b
003275:  66 3B 4D 08                  cmp      ecx, dword ptr [di + 8]
003279:  7F EE                        jg       0x3269
00327B:  89 37                        mov      word ptr [bx], si
00327D:  89 5C 10                     mov      word ptr [si + 0x10], bx
003280:  89 3C                        mov      word ptr [si], di
003282:  89 75 10                     mov      word ptr [di + 0x10], si
003285:  C3                           ret     
003286:  C3                           ret     
