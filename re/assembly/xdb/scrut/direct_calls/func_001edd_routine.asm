; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001edd
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 591
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 6f5317ac95a203f579dc60dd859573d7eb7f965bc22fc5298ade3e47b1ae2511

001EDD:  A1 FA 22                     mov      ax, word ptr [0x22fa]
001EE0:  8B 36 F6 22                  mov      si, word ptr [0x22f6]
001EE4:  8B 3E F8 22                  mov      di, word ptr [0x22f8]
001EE8:  25 FC 0F                     and      ax, 0xffc
001EEB:  81 E6 FC 0F                  and      si, 0xffc
001EEF:  81 E7 FC 0F                  and      di, 0xffc
001EF3:  89 3E 30 00                  mov      word ptr [0x30], di
001EF7:  89 36 32 00                  mov      word ptr [0x32], si
001EFB:  A3 34 00                     mov      word ptr [0x34], ax
001EFE:  03 F8                        add      di, ax
001F00:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001F06:  2B F7                        sub      si, di
001F08:  66 03 D2                     add      edx, edx
001F0B:  81 E6 FC 0F                  and      si, 0xffc
001F0F:  66 F7 DA                     neg      edx
001F12:  66 89 16 A0 22               mov      dword ptr [0x22a0], edx
001F17:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
001F1D:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
001F23:  03 F7                        add      si, di
001F25:  03 F7                        add      si, di
001F27:  81 E6 FC 0F                  and      si, 0xffc
001F2B:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001F31:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001F37:  66 2B C1                     sub      eax, ecx
001F3A:  66 03 EA                     add      ebp, edx
001F3D:  66 D1 F8                     sar      eax, 1
001F40:  66 D1 FD                     sar      ebp, 1
001F43:  81 E7 FC 0F                  and      di, 0xffc
001F47:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
001F4D:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
001F53:  66 03 C1                     add      eax, ecx
001F56:  66 03 EA                     add      ebp, edx
001F59:  66 A3 90 22                  mov      dword ptr [0x2290], eax
001F5D:  66 F7 D8                     neg      eax
001F60:  66 A3 8C 22                  mov      dword ptr [0x228c], eax
001F64:  66 89 2E 84 22               mov      dword ptr [0x2284], ebp
001F69:  66 89 2E 98 22               mov      dword ptr [0x2298], ebp
001F6E:  8B 3E 30 00                  mov      di, word ptr [0x30]
001F72:  2B 3E 34 00                  sub      di, word ptr [0x34]
001F76:  8B 36 32 00                  mov      si, word ptr [0x32]
001F7A:  2B F7                        sub      si, di
001F7C:  81 E6 FC 0F                  and      si, 0xffc
001F80:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
001F86:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
001F8C:  03 F7                        add      si, di
001F8E:  03 F7                        add      si, di
001F90:  81 E6 FC 0F                  and      si, 0xffc
001F94:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001F9A:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
001FA0:  66 2B C1                     sub      eax, ecx
001FA3:  66 03 EA                     add      ebp, edx
001FA6:  66 D1 F8                     sar      eax, 1
001FA9:  66 D1 FD                     sar      ebp, 1
001FAC:  81 E7 FC 0F                  and      di, 0xffc
001FB0:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
001FB6:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
001FBC:  66 2B C8                     sub      ecx, eax
001FBF:  66 2B D5                     sub      edx, ebp
001FC2:  66 29 0E 90 22               sub      dword ptr [0x2290], ecx
001FC7:  66 29 0E 8C 22               sub      dword ptr [0x228c], ecx
001FCC:  66 01 16 84 22               add      dword ptr [0x2284], edx
001FD1:  66 29 16 98 22               sub      dword ptr [0x2298], edx
001FD6:  8B 3E 34 00                  mov      di, word ptr [0x34]
001FDA:  8B 2E 32 00                  mov      bp, word ptr [0x32]
001FDE:  8B F7                        mov      si, di
001FE0:  03 FD                        add      di, bp
001FE2:  2B F5                        sub      si, bp
001FE4:  81 E7 FC 0F                  and      di, 0xffc
001FE8:  81 E6 FC 0F                  and      si, 0xffc
001FEC:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
001FF2:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
001FF8:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
001FFE:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002004:  66 03 C1                     add      eax, ecx
002007:  66 03 EA                     add      ebp, edx
00200A:  66 F7 DD                     neg      ebp
00200D:  66 A3 94 22                  mov      dword ptr [0x2294], eax
002011:  66 89 2E 88 22               mov      dword ptr [0x2288], ebp
002016:  8B 3E 30 00                  mov      di, word ptr [0x30]
00201A:  8B 2E 32 00                  mov      bp, word ptr [0x32]
00201E:  8B F7                        mov      si, di
002020:  03 FD                        add      di, bp
002022:  2B F5                        sub      si, bp
002024:  81 E7 FC 0F                  and      di, 0xffc
002028:  81 E6 FC 0F                  and      si, 0xffc
00202C:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
002032:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
002038:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
00203E:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002044:  66 03 C1                     add      eax, ecx
002047:  66 03 EA                     add      ebp, edx
00204A:  66 A3 A4 22                  mov      dword ptr [0x22a4], eax
00204E:  66 89 2E 9C 22               mov      dword ptr [0x229c], ebp
002053:  BE 84 22                     mov      si, 0x2284
002056:  BF BA 22                     mov      di, 0x22ba
002059:  B9 09 00                     mov      cx, 9
00205C:  66 8B 04                     mov      eax, dword ptr [si]
00205F:  83 C6 04                     add      si, 4
002062:  66 2B 05                     sub      eax, dword ptr [di]
002065:  66 C1 F8 03                  sar      eax, 3
002069:  66 11 05                     adc      dword ptr [di], eax
00206C:  83 C7 04                     add      di, 4
00206F:  E2 EB                        loop     0x205c
002071:  66 0F BF 1E FC 22            movsx    ebx, word ptr [0x22fc]
002077:  66 F7 DB                     neg      ebx
00207A:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
00207E:  66 F7 EB                     imul     ebx
002081:  66 C1 F8 03                  sar      eax, 3
002085:  66 01 06 EA 22               add      dword ptr [0x22ea], eax
00208A:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
00208E:  66 F7 EB                     imul     ebx
002091:  66 C1 F8 03                  sar      eax, 3
002095:  66 01 06 EE 22               add      dword ptr [0x22ee], eax
00209A:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
00209E:  66 F7 EB                     imul     ebx
0020A1:  66 C1 F8 03                  sar      eax, 3
0020A5:  66 01 06 F2 22               add      dword ptr [0x22f2], eax
0020AA:  66 0F BF 1E EC 22            movsx    ebx, word ptr [0x22ec]
0020B0:  66 0F BF 0E F0 22            movsx    ecx, word ptr [0x22f0]
0020B6:  66 0F BF 36 F4 22            movsx    esi, word ptr [0x22f4]
0020BC:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
0020C0:  66 0F AF C3                  imul     eax, ebx
0020C4:  66 8B E8                     mov      ebp, eax
0020C7:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
0020CB:  66 0F AF C1                  imul     eax, ecx
0020CF:  66 03 E8                     add      ebp, eax
0020D2:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
0020D6:  66 0F AF C6                  imul     eax, esi
0020DA:  66 03 C5                     add      eax, ebp
0020DD:  66 A3 E6 22                  mov      dword ptr [0x22e6], eax
0020E1:  66 A1 C6 22                  mov      eax, dword ptr [0x22c6]
0020E5:  66 0F AF C3                  imul     eax, ebx
0020E9:  66 8B E8                     mov      ebp, eax
0020EC:  66 A1 CA 22                  mov      eax, dword ptr [0x22ca]
0020F0:  66 0F AF C1                  imul     eax, ecx
0020F4:  66 03 E8                     add      ebp, eax
0020F7:  66 A1 CE 22                  mov      eax, dword ptr [0x22ce]
0020FB:  66 0F AF C6                  imul     eax, esi
0020FF:  66 03 C5                     add      eax, ebp
002102:  66 A3 E2 22                  mov      dword ptr [0x22e2], eax
002106:  66 A1 BA 22                  mov      eax, dword ptr [0x22ba]
00210A:  66 0F AF C3                  imul     eax, ebx
00210E:  66 8B E8                     mov      ebp, eax
002111:  66 A1 BE 22                  mov      eax, dword ptr [0x22be]
002115:  66 0F AF C1                  imul     eax, ecx
002119:  66 03 E8                     add      ebp, eax
00211C:  66 A1 C2 22                  mov      eax, dword ptr [0x22c2]
002120:  66 0F AF C6                  imul     eax, esi
002124:  66 03 C5                     add      eax, ebp
002127:  66 A3 DE 22                  mov      dword ptr [0x22de], eax
00212B:  C3                           ret     
