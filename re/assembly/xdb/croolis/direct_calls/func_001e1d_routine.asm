; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001e1d
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 591
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/croolis/direct_calls/func_001e1d_routine.cpp
; routine_bytes_sha256: 6f5317ac95a203f579dc60dd859573d7eb7f965bc22fc5298ade3e47b1ae2511

001E1D:  A1 FA 22                     mov      ax, word ptr [0x22fa]
001E20:  8B 36 F6 22                  mov      si, word ptr [0x22f6]
001E24:  8B 3E F8 22                  mov      di, word ptr [0x22f8]
001E28:  25 FC 0F                     and      ax, 0xffc
001E2B:  81 E6 FC 0F                  and      si, 0xffc
001E2F:  81 E7 FC 0F                  and      di, 0xffc
001E33:  89 3E 30 00                  mov      word ptr [0x30], di
001E37:  89 36 32 00                  mov      word ptr [0x32], si
001E3B:  A3 34 00                     mov      word ptr [0x34], ax
001E3E:  03 F8                        add      di, ax
001E40:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001E46:  2B F7                        sub      si, di
001E48:  66 03 D2                     add      edx, edx
001E4B:  81 E6 FC 0F                  and      si, 0xffc
001E4F:  66 F7 DA                     neg      edx
001E52:  66 89 16 A0 22               mov      dword ptr [0x22a0], edx
001E57:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
001E5D:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
001E63:  03 F7                        add      si, di
001E65:  03 F7                        add      si, di
001E67:  81 E6 FC 0F                  and      si, 0xffc
001E6B:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001E71:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001E77:  66 2B C1                     sub      eax, ecx
001E7A:  66 03 EA                     add      ebp, edx
001E7D:  66 D1 F8                     sar      eax, 1
001E80:  66 D1 FD                     sar      ebp, 1
001E83:  81 E7 FC 0F                  and      di, 0xffc
001E87:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
001E8D:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
001E93:  66 03 C1                     add      eax, ecx
001E96:  66 03 EA                     add      ebp, edx
001E99:  66 A3 90 22                  mov      dword ptr [0x2290], eax
001E9D:  66 F7 D8                     neg      eax
001EA0:  66 A3 8C 22                  mov      dword ptr [0x228c], eax
001EA4:  66 89 2E 84 22               mov      dword ptr [0x2284], ebp
001EA9:  66 89 2E 98 22               mov      dword ptr [0x2298], ebp
001EAE:  8B 3E 30 00                  mov      di, word ptr [0x30]
001EB2:  2B 3E 34 00                  sub      di, word ptr [0x34]
001EB6:  8B 36 32 00                  mov      si, word ptr [0x32]
001EBA:  2B F7                        sub      si, di
001EBC:  81 E6 FC 0F                  and      si, 0xffc
001EC0:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
001EC6:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
001ECC:  03 F7                        add      si, di
001ECE:  03 F7                        add      si, di
001ED0:  81 E6 FC 0F                  and      si, 0xffc
001ED4:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001EDA:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001EE0:  66 2B C1                     sub      eax, ecx
001EE3:  66 03 EA                     add      ebp, edx
001EE6:  66 D1 F8                     sar      eax, 1
001EE9:  66 D1 FD                     sar      ebp, 1
001EEC:  81 E7 FC 0F                  and      di, 0xffc
001EF0:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
001EF6:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
001EFC:  66 2B C8                     sub      ecx, eax
001EFF:  66 2B D5                     sub      edx, ebp
001F02:  66 29 0E 90 22               sub      dword ptr [0x2290], ecx
001F07:  66 29 0E 8C 22               sub      dword ptr [0x228c], ecx
001F0C:  66 01 16 84 22               add      dword ptr [0x2284], edx
001F11:  66 29 16 98 22               sub      dword ptr [0x2298], edx
001F16:  8B 3E 34 00                  mov      di, word ptr [0x34]
001F1A:  8B 2E 32 00                  mov      bp, word ptr [0x32]
001F1E:  8B F7                        mov      si, di
001F20:  03 FD                        add      di, bp
001F22:  2B F5                        sub      si, bp
001F24:  81 E7 FC 0F                  and      di, 0xffc
001F28:  81 E6 FC 0F                  and      si, 0xffc
001F2C:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
001F32:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
001F38:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001F3E:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001F44:  66 03 C1                     add      eax, ecx
001F47:  66 03 EA                     add      ebp, edx
001F4A:  66 F7 DD                     neg      ebp
001F4D:  66 A3 94 22                  mov      dword ptr [0x2294], eax
001F51:  66 89 2E 88 22               mov      dword ptr [0x2288], ebp
001F56:  8B 3E 30 00                  mov      di, word ptr [0x30]
001F5A:  8B 2E 32 00                  mov      bp, word ptr [0x32]
001F5E:  8B F7                        mov      si, di
001F60:  03 FD                        add      di, bp
001F62:  2B F5                        sub      si, bp
001F64:  81 E7 FC 0F                  and      di, 0xffc
001F68:  81 E6 FC 0F                  and      si, 0xffc
001F6C:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
001F72:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
001F78:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001F7E:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001F84:  66 03 C1                     add      eax, ecx
001F87:  66 03 EA                     add      ebp, edx
001F8A:  66 A3 A4 22                  mov      dword ptr [0x22a4], eax
001F8E:  66 89 2E 9C 22               mov      dword ptr [0x229c], ebp
001F93:  BE 84 22                     mov      si, 0x2284
001F96:  BF BA 22                     mov      di, 0x22ba
001F99:  B9 09 00                     mov      cx, 9
001F9C:  66 8B 04                     mov      eax, dword ptr [si]
001F9F:  83 C6 04                     add      si, 4
001FA2:  66 2B 05                     sub      eax, dword ptr [di]
001FA5:  66 C1 F8 03                  sar      eax, 3
001FA9:  66 11 05                     adc      dword ptr [di], eax
001FAC:  83 C7 04                     add      di, 4
001FAF:  E2 EB                        loop     0x1f9c
001FB1:  66 0F BF 1E FC 22            movsx    ebx, word ptr [0x22fc]
001FB7:  66 F7 DB                     neg      ebx
001FBA:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
001FBE:  66 F7 EB                     imul     ebx
001FC1:  66 C1 F8 03                  sar      eax, 3
001FC5:  66 01 06 EA 22               add      dword ptr [0x22ea], eax
001FCA:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
001FCE:  66 F7 EB                     imul     ebx
001FD1:  66 C1 F8 03                  sar      eax, 3
001FD5:  66 01 06 EE 22               add      dword ptr [0x22ee], eax
001FDA:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
001FDE:  66 F7 EB                     imul     ebx
001FE1:  66 C1 F8 03                  sar      eax, 3
001FE5:  66 01 06 F2 22               add      dword ptr [0x22f2], eax
001FEA:  66 0F BF 1E EC 22            movsx    ebx, word ptr [0x22ec]
001FF0:  66 0F BF 0E F0 22            movsx    ecx, word ptr [0x22f0]
001FF6:  66 0F BF 36 F4 22            movsx    esi, word ptr [0x22f4]
001FFC:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
002000:  66 0F AF C3                  imul     eax, ebx
002004:  66 8B E8                     mov      ebp, eax
002007:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
00200B:  66 0F AF C1                  imul     eax, ecx
00200F:  66 03 E8                     add      ebp, eax
002012:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
002016:  66 0F AF C6                  imul     eax, esi
00201A:  66 03 C5                     add      eax, ebp
00201D:  66 A3 E6 22                  mov      dword ptr [0x22e6], eax
002021:  66 A1 C6 22                  mov      eax, dword ptr [0x22c6]
002025:  66 0F AF C3                  imul     eax, ebx
002029:  66 8B E8                     mov      ebp, eax
00202C:  66 A1 CA 22                  mov      eax, dword ptr [0x22ca]
002030:  66 0F AF C1                  imul     eax, ecx
002034:  66 03 E8                     add      ebp, eax
002037:  66 A1 CE 22                  mov      eax, dword ptr [0x22ce]
00203B:  66 0F AF C6                  imul     eax, esi
00203F:  66 03 C5                     add      eax, ebp
002042:  66 A3 E2 22                  mov      dword ptr [0x22e2], eax
002046:  66 A1 BA 22                  mov      eax, dword ptr [0x22ba]
00204A:  66 0F AF C3                  imul     eax, ebx
00204E:  66 8B E8                     mov      ebp, eax
002051:  66 A1 BE 22                  mov      eax, dword ptr [0x22be]
002055:  66 0F AF C1                  imul     eax, ecx
002059:  66 03 E8                     add      ebp, eax
00205C:  66 A1 C2 22                  mov      eax, dword ptr [0x22c2]
002060:  66 0F AF C6                  imul     eax, esi
002064:  66 03 C5                     add      eax, ebp
002067:  66 A3 DE 22                  mov      dword ptr [0x22de], eax
00206B:  C3                           ret     
