; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001dd8
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 591
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 6f5317ac95a203f579dc60dd859573d7eb7f965bc22fc5298ade3e47b1ae2511

001DD8:  A1 FA 22                     mov      ax, word ptr [0x22fa]
001DDB:  8B 36 F6 22                  mov      si, word ptr [0x22f6]
001DDF:  8B 3E F8 22                  mov      di, word ptr [0x22f8]
001DE3:  25 FC 0F                     and      ax, 0xffc
001DE6:  81 E6 FC 0F                  and      si, 0xffc
001DEA:  81 E7 FC 0F                  and      di, 0xffc
001DEE:  89 3E 30 00                  mov      word ptr [0x30], di
001DF2:  89 36 32 00                  mov      word ptr [0x32], si
001DF6:  A3 34 00                     mov      word ptr [0x34], ax
001DF9:  03 F8                        add      di, ax
001DFB:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001E01:  2B F7                        sub      si, di
001E03:  66 03 D2                     add      edx, edx
001E06:  81 E6 FC 0F                  and      si, 0xffc
001E0A:  66 F7 DA                     neg      edx
001E0D:  66 89 16 A0 22               mov      dword ptr [0x22a0], edx
001E12:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
001E18:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
001E1E:  03 F7                        add      si, di
001E20:  03 F7                        add      si, di
001E22:  81 E6 FC 0F                  and      si, 0xffc
001E26:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001E2C:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001E32:  66 2B C1                     sub      eax, ecx
001E35:  66 03 EA                     add      ebp, edx
001E38:  66 D1 F8                     sar      eax, 1
001E3B:  66 D1 FD                     sar      ebp, 1
001E3E:  81 E7 FC 0F                  and      di, 0xffc
001E42:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
001E48:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
001E4E:  66 03 C1                     add      eax, ecx
001E51:  66 03 EA                     add      ebp, edx
001E54:  66 A3 90 22                  mov      dword ptr [0x2290], eax
001E58:  66 F7 D8                     neg      eax
001E5B:  66 A3 8C 22                  mov      dword ptr [0x228c], eax
001E5F:  66 89 2E 84 22               mov      dword ptr [0x2284], ebp
001E64:  66 89 2E 98 22               mov      dword ptr [0x2298], ebp
001E69:  8B 3E 30 00                  mov      di, word ptr [0x30]
001E6D:  2B 3E 34 00                  sub      di, word ptr [0x34]
001E71:  8B 36 32 00                  mov      si, word ptr [0x32]
001E75:  2B F7                        sub      si, di
001E77:  81 E6 FC 0F                  and      si, 0xffc
001E7B:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
001E81:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
001E87:  03 F7                        add      si, di
001E89:  03 F7                        add      si, di
001E8B:  81 E6 FC 0F                  and      si, 0xffc
001E8F:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001E95:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001E9B:  66 2B C1                     sub      eax, ecx
001E9E:  66 03 EA                     add      ebp, edx
001EA1:  66 D1 F8                     sar      eax, 1
001EA4:  66 D1 FD                     sar      ebp, 1
001EA7:  81 E7 FC 0F                  and      di, 0xffc
001EAB:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
001EB1:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
001EB7:  66 2B C8                     sub      ecx, eax
001EBA:  66 2B D5                     sub      edx, ebp
001EBD:  66 29 0E 90 22               sub      dword ptr [0x2290], ecx
001EC2:  66 29 0E 8C 22               sub      dword ptr [0x228c], ecx
001EC7:  66 01 16 84 22               add      dword ptr [0x2284], edx
001ECC:  66 29 16 98 22               sub      dword ptr [0x2298], edx
001ED1:  8B 3E 34 00                  mov      di, word ptr [0x34]
001ED5:  8B 2E 32 00                  mov      bp, word ptr [0x32]
001ED9:  8B F7                        mov      si, di
001EDB:  03 FD                        add      di, bp
001EDD:  2B F5                        sub      si, bp
001EDF:  81 E7 FC 0F                  and      di, 0xffc
001EE3:  81 E6 FC 0F                  and      si, 0xffc
001EE7:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
001EED:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
001EF3:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001EF9:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001EFF:  66 03 C1                     add      eax, ecx
001F02:  66 03 EA                     add      ebp, edx
001F05:  66 F7 DD                     neg      ebp
001F08:  66 A3 94 22                  mov      dword ptr [0x2294], eax
001F0C:  66 89 2E 88 22               mov      dword ptr [0x2288], ebp
001F11:  8B 3E 30 00                  mov      di, word ptr [0x30]
001F15:  8B 2E 32 00                  mov      bp, word ptr [0x32]
001F19:  8B F7                        mov      si, di
001F1B:  03 FD                        add      di, bp
001F1D:  2B F5                        sub      si, bp
001F1F:  81 E7 FC 0F                  and      di, 0xffc
001F23:  81 E6 FC 0F                  and      si, 0xffc
001F27:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
001F2D:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
001F33:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001F39:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001F3F:  66 03 C1                     add      eax, ecx
001F42:  66 03 EA                     add      ebp, edx
001F45:  66 A3 A4 22                  mov      dword ptr [0x22a4], eax
001F49:  66 89 2E 9C 22               mov      dword ptr [0x229c], ebp
001F4E:  BE 84 22                     mov      si, 0x2284
001F51:  BF BA 22                     mov      di, 0x22ba
001F54:  B9 09 00                     mov      cx, 9
001F57:  66 8B 04                     mov      eax, dword ptr [si]
001F5A:  83 C6 04                     add      si, 4
001F5D:  66 2B 05                     sub      eax, dword ptr [di]
001F60:  66 C1 F8 03                  sar      eax, 3
001F64:  66 11 05                     adc      dword ptr [di], eax
001F67:  83 C7 04                     add      di, 4
001F6A:  E2 EB                        loop     0x1f57
001F6C:  66 0F BF 1E FC 22            movsx    ebx, word ptr [0x22fc]
001F72:  66 F7 DB                     neg      ebx
001F75:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
001F79:  66 F7 EB                     imul     ebx
001F7C:  66 C1 F8 03                  sar      eax, 3
001F80:  66 01 06 EA 22               add      dword ptr [0x22ea], eax
001F85:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
001F89:  66 F7 EB                     imul     ebx
001F8C:  66 C1 F8 03                  sar      eax, 3
001F90:  66 01 06 EE 22               add      dword ptr [0x22ee], eax
001F95:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
001F99:  66 F7 EB                     imul     ebx
001F9C:  66 C1 F8 03                  sar      eax, 3
001FA0:  66 01 06 F2 22               add      dword ptr [0x22f2], eax
001FA5:  66 0F BF 1E EC 22            movsx    ebx, word ptr [0x22ec]
001FAB:  66 0F BF 0E F0 22            movsx    ecx, word ptr [0x22f0]
001FB1:  66 0F BF 36 F4 22            movsx    esi, word ptr [0x22f4]
001FB7:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
001FBB:  66 0F AF C3                  imul     eax, ebx
001FBF:  66 8B E8                     mov      ebp, eax
001FC2:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
001FC6:  66 0F AF C1                  imul     eax, ecx
001FCA:  66 03 E8                     add      ebp, eax
001FCD:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
001FD1:  66 0F AF C6                  imul     eax, esi
001FD5:  66 03 C5                     add      eax, ebp
001FD8:  66 A3 E6 22                  mov      dword ptr [0x22e6], eax
001FDC:  66 A1 C6 22                  mov      eax, dword ptr [0x22c6]
001FE0:  66 0F AF C3                  imul     eax, ebx
001FE4:  66 8B E8                     mov      ebp, eax
001FE7:  66 A1 CA 22                  mov      eax, dword ptr [0x22ca]
001FEB:  66 0F AF C1                  imul     eax, ecx
001FEF:  66 03 E8                     add      ebp, eax
001FF2:  66 A1 CE 22                  mov      eax, dword ptr [0x22ce]
001FF6:  66 0F AF C6                  imul     eax, esi
001FFA:  66 03 C5                     add      eax, ebp
001FFD:  66 A3 E2 22                  mov      dword ptr [0x22e2], eax
002001:  66 A1 BA 22                  mov      eax, dword ptr [0x22ba]
002005:  66 0F AF C3                  imul     eax, ebx
002009:  66 8B E8                     mov      ebp, eax
00200C:  66 A1 BE 22                  mov      eax, dword ptr [0x22be]
002010:  66 0F AF C1                  imul     eax, ecx
002014:  66 03 E8                     add      ebp, eax
002017:  66 A1 C2 22                  mov      eax, dword ptr [0x22c2]
00201B:  66 0F AF C6                  imul     eax, esi
00201F:  66 03 C5                     add      eax, ebp
002022:  66 A3 DE 22                  mov      dword ptr [0x22de], eax
002026:  C3                           ret     
