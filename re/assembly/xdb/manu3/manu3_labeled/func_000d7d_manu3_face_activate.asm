; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000d7d
; group: manu3_labeled
; provenance: direct_call_from_0x6f6, direct_call_from_0x700, internal_label:manu3_gradient_setup, label:manu3_face_activate, manu3 face activation and gradient setup
; label: manu3_face_activate
; label_comment: per-face activation (called per bucket face from 0x8C6): converts a face into EDGE records with linear interpolators {value = base(+0x20) + coord(+0xA)*step(+0x28)} — the u/v/depth gradient setup lives here (next slice). Edge flags +0x1A bit15; per-edge eval at 0x90C (eax = -coord*step+base)
; byte_count: 1514
; boundary: cfg_blocks_30_terminals_7
; terminal: ret:7
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 823c014f74d7371b875944a9ae293253654327074a745ec5605fdea15c3aa1a5

000D7D:  26 8B 5C 02                  mov      bx, word ptr es:[si + 2]
000D81:  26 8B 7C 04                  mov      di, word ptr es:[si + 4]
000D85:  26 8B 6C 06                  mov      bp, word ptr es:[si + 6]
000D89:  8B 36 08 09                  mov      si, word ptr [0x908]
000D8D:  0B F6                        or       si, si
000D8F:  0F 84 B5 FA                  je       0x848
000D93:  66 26 8B 47 0A               mov      eax, dword ptr es:[bx + 0xa]
000D98:  66 26 8B 55 0A               mov      edx, dword ptr es:[di + 0xa]
000D9D:  66 26 8B 4E 0A               mov      ecx, dword ptr es:[bp + 0xa]
000DA2:  66 89 16 24 06               mov      dword ptr [0x624], edx
000DA7:  66 89 0E 2C 06               mov      dword ptr [0x62c], ecx
000DAC:  2B D0                        sub      dx, ax
000DAE:  0F 84 DD 03                  je       0x118f
000DB2:  2B C8                        sub      cx, ax
000DB4:  0F 84 AE 05                  je       0x1366
000DB8:  66 A3 1C 06                  mov      dword ptr [0x61c], eax
000DBC:  33 C0                        xor      ax, ax
000DBE:  89 1E 70 06                  mov      word ptr [0x670], bx
000DC2:  8B DA                        mov      bx, dx
000DC4:  89 3E 72 06                  mov      word ptr [0x672], di
000DC8:  C1 E3 02                     shl      bx, 2
000DCB:  89 2E 74 06                  mov      word ptr [0x674], bp
000DCF:  66 8B 3F                     mov      edi, dword ptr [bx]
000DD2:  8B D9                        mov      bx, cx
000DD4:  C1 E3 02                     shl      bx, 2
000DD7:  49                           dec      cx
000DD8:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
000DDB:  66 8B 1F                     mov      ebx, dword ptr [bx]
000DDE:  66 2B D0                     sub      edx, eax
000DE1:  66 2B C8                     sub      ecx, eax
000DE4:  66 C1 FA 10                  sar      edx, 0x10
000DE8:  66 C1 F9 10                  sar      ecx, 0x10
000DEC:  66 0F AF D7                  imul     edx, edi
000DF0:  66 0F AF CB                  imul     ecx, ebx
000DF4:  66 8B E9                     mov      ebp, ecx
000DF7:  66 2B EA                     sub      ebp, edx
000DFA:  0F 8D 90 03                  jge      0x118e
000DFE:  66 C1 FD 08                  sar      ebp, 8
000E02:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
000E06:  66 F7 DD                     neg      ebp
000E09:  66 89 54 1C                  mov      dword ptr [si + 0x1c], edx
000E0D:  66 D1 F9                     sar      ecx, 1
000E10:  66 D1 FA                     sar      edx, 1
000E13:  66 03 C8                     add      ecx, eax
000E16:  66 03 D0                     add      edx, eax
000E19:  66 89 4C 08                  mov      dword ptr [si + 8], ecx
000E1D:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
000E21:  66 89 3E 76 06               mov      dword ptr [0x676], edi
000E26:  66 89 1E 7A 06               mov      dword ptr [0x67a], ebx
000E2B:  8B 1E 70 06                  mov      bx, word ptr [0x670]
000E2F:  66 26 8B 17                  mov      edx, dword ptr es:[bx]
000E33:  8B 1E 74 06                  mov      bx, word ptr [0x674]
000E37:  66 89 16 20 06               mov      dword ptr [0x620], edx
000E3C:  66 26 8B 0F                  mov      ecx, dword ptr es:[bx]
000E40:  8B 1E 72 06                  mov      bx, word ptr [0x672]
000E44:  66 89 0E 30 06               mov      dword ptr [0x630], ecx
000E49:  66 26 8B 1F                  mov      ebx, dword ptr es:[bx]
000E4D:  66 89 1E 28 06               mov      dword ptr [0x628], ebx
000E52:  2B DA                        sub      bx, dx
000E54:  2B CA                        sub      cx, dx
000E56:  66 0F BF DB                  movsx    ebx, bx
000E5A:  66 0F BF C9                  movsx    ecx, cx
000E5E:  66 0F AF 1E 76 06            imul     ebx, dword ptr [0x676]
000E64:  66 0F AF 0E 7A 06            imul     ecx, dword ptr [0x67a]
000E6A:  66 8B C3                     mov      eax, ebx
000E6D:  66 2B C1                     sub      eax, ecx
000E70:  66 C1 F9 08                  sar      ecx, 8
000E74:  C1 E2 08                     shl      dx, 8
000E77:  89 4C 4A                     mov      word ptr [si + 0x4a], cx
000E7A:  D1 F9                        sar      cx, 1
000E7C:  03 D1                        add      dx, cx
000E7E:  89 54 42                     mov      word ptr [si + 0x42], dx
000E81:  66 99                        cdq
000E83:  66 F7 FD                     idiv     ebp
000E86:  89 44 52                     mov      word ptr [si + 0x52], ax
000E89:  66 0F B7 16 22 06            movzx    edx, word ptr [0x622]
000E8F:  B0 00                        mov      al, 0
000E91:  66 0F B7 1E 2A 06            movzx    ebx, word ptr [0x62a]
000E97:  8A E6                        mov      ah, dh
000E99:  66 0F B7 0E 32 06            movzx    ecx, word ptr [0x632]
000E9F:  C1 E0 04                     shl      ax, 4
000EA2:  66 2B DA                     sub      ebx, edx
000EA5:  64 03 06 04 00               add      ax, word ptr fs:[4]
000EAA:  66 2B CA                     sub      ecx, edx
000EAD:  66 0F AF 1E 76 06            imul     ebx, dword ptr [0x676]
000EB3:  89 44 56                     mov      word ptr [si + 0x56], ax
000EB6:  66 0F AF 0E 7A 06            imul     ecx, dword ptr [0x67a]
000EBC:  66 8B C3                     mov      eax, ebx
000EBF:  66 2B C1                     sub      eax, ecx
000EC2:  66 C1 F9 08                  sar      ecx, 8
000EC6:  C1 E2 08                     shl      dx, 8
000EC9:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
000ECC:  D1 F9                        sar      cx, 1
000ECE:  03 D1                        add      dx, cx
000ED0:  89 54 44                     mov      word ptr [si + 0x44], dx
000ED3:  66 99                        cdq
000ED5:  66 F7 FD                     idiv     ebp
000ED8:  89 44 54                     mov      word ptr [si + 0x54], ax
000EDB:  8B 1E 70 06                  mov      bx, word ptr [0x670]
000EDF:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
000EE4:  8B 1E 74 06                  mov      bx, word ptr [0x674]
000EE8:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
000EED:  66 2B C1                     sub      eax, ecx
000EF0:  66 F7 2E 7A 06               imul     dword ptr [0x67a]
000EF5:  66 0F AC D0 10               shrd     eax, edx, 0x10
000EFA:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
000EFE:  66 D1 F8                     sar      eax, 1
000F01:  66 03 C8                     add      ecx, eax
000F04:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
000F08:  8B 1E 70 06                  mov      bx, word ptr [0x670]
000F0C:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
000F11:  8B 1E 72 06                  mov      bx, word ptr [0x672]
000F15:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
000F1A:  8B 1E 74 06                  mov      bx, word ptr [0x674]
000F1E:  66 26 8B 5F 0E               mov      ebx, dword ptr es:[bx + 0xe]
000F23:  66 2B C8                     sub      ecx, eax
000F26:  66 2B D8                     sub      ebx, eax
000F29:  66 0F AF 0E 76 06            imul     ecx, dword ptr [0x676]
000F2F:  66 0F AF 1E 7A 06            imul     ebx, dword ptr [0x67a]
000F35:  66 8B C1                     mov      eax, ecx
000F38:  66 2B C3                     sub      eax, ebx
000F3B:  66 99                        cdq
000F3D:  66 F7 FD                     idiv     ebp
000F40:  66 C1 F8 08                  sar      eax, 8
000F44:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
000F48:  A1 2C 06                     mov      ax, word ptr [0x62c]
000F4B:  8B 1E 24 06                  mov      bx, word ptr [0x624]
000F4F:  2B D8                        sub      bx, ax
000F51:  0F 88 E9 01                  js       0x113e
000F55:  0F 84 82 03                  je       0x12db
000F59:  0B C0                        or       ax, ax
000F5B:  0F 88 AA 00                  js       0x1009
000F5F:  4B                           dec      bx
000F60:  89 5C 30                     mov      word ptr [si + 0x30], bx
000F63:  C1 E3 02                     shl      bx, 2
000F66:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
000F6A:  66 0F BF 06 28 06            movsx    eax, word ptr [0x628]
000F70:  66 0F BF 1E 30 06            movsx    ebx, word ptr [0x630]
000F76:  66 0F BF 0E 2A 06            movsx    ecx, word ptr [0x62a]
000F7C:  66 0F BF 16 32 06            movsx    edx, word ptr [0x632]
000F82:  66 2B C3                     sub      eax, ebx
000F85:  66 2B CA                     sub      ecx, edx
000F88:  C1 E3 08                     shl      bx, 8
000F8B:  C1 E2 08                     shl      dx, 8
000F8E:  66 0F AF C5                  imul     eax, ebp
000F92:  66 0F AF CD                  imul     ecx, ebp
000F96:  66 C1 F8 08                  sar      eax, 8
000F9A:  66 C1 F9 08                  sar      ecx, 8
000F9E:  89 44 4E                     mov      word ptr [si + 0x4e], ax
000FA1:  89 4C 50                     mov      word ptr [si + 0x50], cx
000FA4:  D1 F8                        sar      ax, 1
000FA6:  D1 F9                        sar      cx, 1
000FA8:  03 D8                        add      bx, ax
000FAA:  03 D1                        add      dx, cx
000FAC:  89 5C 46                     mov      word ptr [si + 0x46], bx
000FAF:  89 54 48                     mov      word ptr [si + 0x48], dx
000FB2:  66 0F BF 1E 2E 06            movsx    ebx, word ptr [0x62e]
000FB8:  66 0F BF 06 26 06            movsx    eax, word ptr [0x626]
000FBE:  66 2B C3                     sub      eax, ebx
000FC1:  66 C1 E3 10                  shl      ebx, 0x10
000FC5:  66 F7 ED                     imul     ebp
000FC8:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
000FCC:  66 D1 F8                     sar      eax, 1
000FCF:  66 03 D8                     add      ebx, eax
000FD2:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
000FD6:  8B 1E 72 06                  mov      bx, word ptr [0x672]
000FDA:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
000FDF:  8B 1E 74 06                  mov      bx, word ptr [0x674]
000FE3:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
000FE8:  66 2B C1                     sub      eax, ecx
000FEB:  66 F7 ED                     imul     ebp
000FEE:  66 0F AC D0 10               shrd     eax, edx, 0x10
000FF3:  66 89 44 3E                  mov      dword ptr [si + 0x3e], eax
000FF7:  66 D1 F8                     sar      eax, 1
000FFA:  66 03 C8                     add      ecx, eax
000FFD:  66 89 4C 3A                  mov      dword ptr [si + 0x3a], ecx
001001:  C7 44 2C CA 0C               mov      word ptr [si + 0x2c], 0xcca
001006:  E9 78 01                     jmp      0x1181
001009:  4B                           dec      bx
00100A:  66 0F B7 F8                  movzx    edi, ax
00100E:  03 C3                        add      ax, bx
001010:  F7 DF                        neg      di
001012:  89 44 2E                     mov      word ptr [si + 0x2e], ax
001015:  C1 E3 02                     shl      bx, 2
001018:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
00101C:  66 0F BF 06 28 06            movsx    eax, word ptr [0x628]
001022:  66 0F BF 1E 30 06            movsx    ebx, word ptr [0x630]
001028:  66 0F BF 0E 2A 06            movsx    ecx, word ptr [0x62a]
00102E:  66 0F BF 16 32 06            movsx    edx, word ptr [0x632]
001034:  66 2B C3                     sub      eax, ebx
001037:  66 2B CA                     sub      ecx, edx
00103A:  C1 E3 08                     shl      bx, 8
00103D:  C1 E2 08                     shl      dx, 8
001040:  66 0F AF C5                  imul     eax, ebp
001044:  66 0F AF CD                  imul     ecx, ebp
001048:  66 C1 F8 08                  sar      eax, 8
00104C:  66 C1 F9 08                  sar      ecx, 8
001050:  89 44 4A                     mov      word ptr [si + 0x4a], ax
001053:  89 4C 4C                     mov      word ptr [si + 0x4c], cx
001056:  0F AF C7                     imul     ax, di
001059:  0F AF CF                     imul     cx, di
00105C:  03 D8                        add      bx, ax
00105E:  03 D1                        add      dx, cx
001060:  89 5C 42                     mov      word ptr [si + 0x42], bx
001063:  89 54 44                     mov      word ptr [si + 0x44], dx
001066:  66 0F BF 1E 2E 06            movsx    ebx, word ptr [0x62e]
00106C:  66 0F BF 06 26 06            movsx    eax, word ptr [0x626]
001072:  66 2B C3                     sub      eax, ebx
001075:  66 C1 E3 10                  shl      ebx, 0x10
001079:  66 F7 ED                     imul     ebp
00107C:  66 89 44 0C                  mov      dword ptr [si + 0xc], eax
001080:  66 0F AF C7                  imul     eax, edi
001084:  66 03 D8                     add      ebx, eax
001087:  66 89 5C 08                  mov      dword ptr [si + 8], ebx
00108B:  8B 1E 72 06                  mov      bx, word ptr [0x672]
00108F:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
001094:  8B 1E 74 06                  mov      bx, word ptr [0x674]
001098:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
00109D:  66 2B C1                     sub      eax, ecx
0010A0:  66 F7 ED                     imul     ebp
0010A3:  66 0F AC D0 10               shrd     eax, edx, 0x10
0010A8:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
0010AC:  66 0F AF C7                  imul     eax, edi
0010B0:  66 03 C8                     add      ecx, eax
0010B3:  66 89 4C 20                  mov      dword ptr [si + 0x20], ecx
0010B7:  66 0F B7 1E 1C 06            movzx    ebx, word ptr [0x61c]
0010BD:  F7 DB                        neg      bx
0010BF:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
0010C3:  66 0F AF CB                  imul     ecx, ebx
0010C7:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
0010CB:  C7 44 2C 5E 0D               mov      word ptr [si + 0x2c], 0xd5e
0010D0:  E9 56 02                     jmp      0x1329
0010D3:  4B                           dec      bx
0010D4:  F7 DF                        neg      di
0010D6:  C1 E3 02                     shl      bx, 2
0010D9:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
0010DD:  66 0F BF 1E 26 06            movsx    ebx, word ptr [0x626]
0010E3:  66 0F BF 06 2E 06            movsx    eax, word ptr [0x62e]
0010E9:  66 2B C3                     sub      eax, ebx
0010EC:  66 C1 E3 10                  shl      ebx, 0x10
0010F0:  66 F7 ED                     imul     ebp
0010F3:  66 89 44 1C                  mov      dword ptr [si + 0x1c], eax
0010F7:  66 F7 EF                     imul     edi
0010FA:  66 03 D8                     add      ebx, eax
0010FD:  66 89 5C 18                  mov      dword ptr [si + 0x18], ebx
001101:  66 0F B7 1E 1C 06            movzx    ebx, word ptr [0x61c]
001107:  F7 DB                        neg      bx
001109:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
00110C:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
001110:  66 0F AF C3                  imul     eax, ebx
001114:  66 01 44 08                  add      dword ptr [si + 8], eax
001118:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
00111C:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
00111F:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
001122:  66 0F AF C3                  imul     eax, ebx
001126:  0F AF CB                     imul     cx, bx
001129:  0F AF D3                     imul     dx, bx
00112C:  66 01 44 20                  add      dword ptr [si + 0x20], eax
001130:  01 4C 42                     add      word ptr [si + 0x42], cx
001133:  01 54 44                     add      word ptr [si + 0x44], dx
001136:  C7 44 2C 5E 0D               mov      word ptr [si + 0x2c], 0xd5e
00113B:  E9 EB 01                     jmp      0x1329
00113E:  66 0F B7 3E 24 06            movzx    edi, word ptr [0x624]
001144:  F7 DB                        neg      bx
001146:  0B FF                        or       di, di
001148:  78 89                        js       0x10d3
00114A:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
00114D:  4B                           dec      bx
00114E:  89 5C 30                     mov      word ptr [si + 0x30], bx
001151:  C1 E3 02                     shl      bx, 2
001154:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
001158:  66 0F BF 1E 26 06            movsx    ebx, word ptr [0x626]
00115E:  66 0F BF 06 2E 06            movsx    eax, word ptr [0x62e]
001164:  66 2B C3                     sub      eax, ebx
001167:  66 C1 E3 10                  shl      ebx, 0x10
00116B:  66 F7 ED                     imul     ebp
00116E:  66 89 44 36                  mov      dword ptr [si + 0x36], eax
001172:  66 D1 F8                     sar      eax, 1
001175:  66 03 D8                     add      ebx, eax
001178:  66 89 5C 32                  mov      dword ptr [si + 0x32], ebx
00117C:  C7 44 2C 19 0D               mov      word ptr [si + 0x2c], 0xd19
001181:  F7 06 1C 06 00 80            test     word ptr [0x61c], 0x8000
001187:  0F 84 9E 01                  je       0x1329
00118B:  E9 5A 01                     jmp      0x12e8
00118E:  C3                           ret
00118F:  2B C8                        sub      cx, ax
001191:  74 FB                        je       0x118e
001193:  81 F9 90 01                  cmp      cx, 0x190
001197:  73 F5                        jae      0x118e
001199:  66 A3 1C 06                  mov      dword ptr [0x61c], eax
00119D:  33 C0                        xor      ax, ax
00119F:  66 8B F2                     mov      esi, edx
0011A2:  66 2B F0                     sub      esi, eax
0011A5:  7E E7                        jle      0x118e
0011A7:  66 C1 EE 0E                  shr      esi, 0xe
0011AB:  81 FE 40 06                  cmp      si, 0x640
0011AF:  73 DD                        jae      0x118e
0011B1:  89 1E 70 06                  mov      word ptr [0x670], bx
0011B5:  89 3E 72 06                  mov      word ptr [0x672], di
0011B9:  89 2E 74 06                  mov      word ptr [0x674], bp
0011BD:  8B D9                        mov      bx, cx
0011BF:  66 8B 3C                     mov      edi, dword ptr [si]
0011C2:  C1 E3 02                     shl      bx, 2
0011C5:  49                           dec      cx
0011C6:  8B 36 08 09                  mov      si, word ptr [0x908]
0011CA:  66 8B 2F                     mov      ebp, dword ptr [bx]
0011CD:  89 4C 2E                     mov      word ptr [si + 0x2e], cx
0011D0:  66 8B D9                     mov      ebx, ecx
0011D3:  66 2B C8                     sub      ecx, eax
0011D6:  66 C1 F9 10                  sar      ecx, 0x10
0011DA:  66 0F AF CD                  imul     ecx, ebp
0011DE:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
0011E2:  66 D1 F9                     sar      ecx, 1
0011E5:  66 03 C1                     add      eax, ecx
0011E8:  66 89 44 08                  mov      dword ptr [si + 8], eax
0011EC:  66 2B DA                     sub      ebx, edx
0011EF:  66 C1 FB 10                  sar      ebx, 0x10
0011F3:  66 0F AF DD                  imul     ebx, ebp
0011F7:  66 89 5C 1C                  mov      dword ptr [si + 0x1c], ebx
0011FB:  66 D1 FB                     sar      ebx, 1
0011FE:  66 03 D3                     add      edx, ebx
001201:  66 89 54 18                  mov      dword ptr [si + 0x18], edx
001205:  8B 1E 70 06                  mov      bx, word ptr [0x670]
001209:  B0 00                        mov      al, 0
00120B:  66 26 0F B7 57 02            movzx    edx, word ptr es:[bx + 2]
001211:  8A E6                        mov      ah, dh
001213:  66 26 0F BF 0F               movsx    ecx, word ptr es:[bx]
001218:  C1 E0 04                     shl      ax, 4
00121B:  8B 1E 72 06                  mov      bx, word ptr [0x672]
00121F:  64 03 06 04 00               add      ax, word ptr fs:[4]
001224:  89 44 56                     mov      word ptr [si + 0x56], ax
001227:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
00122C:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
001232:  66 2B C1                     sub      eax, ecx
001235:  66 2B DA                     sub      ebx, edx
001238:  66 0F AF C7                  imul     eax, edi
00123C:  66 0F AF DF                  imul     ebx, edi
001240:  66 C1 F8 08                  sar      eax, 8
001244:  66 C1 FB 08                  sar      ebx, 8
001248:  89 44 52                     mov      word ptr [si + 0x52], ax
00124B:  89 5C 54                     mov      word ptr [si + 0x54], bx
00124E:  8B 1E 74 06                  mov      bx, word ptr [0x674]
001252:  66 26 0F BF 07               movsx    eax, word ptr es:[bx]
001257:  66 26 0F B7 5F 02            movzx    ebx, word ptr es:[bx + 2]
00125D:  66 2B C1                     sub      eax, ecx
001260:  66 2B DA                     sub      ebx, edx
001263:  66 0F AF C5                  imul     eax, ebp
001267:  66 0F AF DD                  imul     ebx, ebp
00126B:  66 C1 F8 08                  sar      eax, 8
00126F:  66 C1 FB 08                  sar      ebx, 8
001273:  89 44 4A                     mov      word ptr [si + 0x4a], ax
001276:  89 5C 4C                     mov      word ptr [si + 0x4c], bx
001279:  C1 E1 08                     shl      cx, 8
00127C:  C1 E2 08                     shl      dx, 8
00127F:  66 D1 F8                     sar      eax, 1
001282:  66 D1 FB                     sar      ebx, 1
001285:  03 C8                        add      cx, ax
001287:  03 D3                        add      dx, bx
001289:  89 4C 42                     mov      word ptr [si + 0x42], cx
00128C:  89 54 44                     mov      word ptr [si + 0x44], dx
00128F:  8B 1E 74 06                  mov      bx, word ptr [0x674]
001293:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
001298:  8B 1E 70 06                  mov      bx, word ptr [0x670]
00129C:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
0012A1:  66 2B C1                     sub      eax, ecx
0012A4:  66 F7 ED                     imul     ebp
0012A7:  66 0F AC D0 10               shrd     eax, edx, 0x10
0012AC:  66 89 44 24                  mov      dword ptr [si + 0x24], eax
0012B0:  66 D1 F8                     sar      eax, 1
0012B3:  66 03 C1                     add      eax, ecx
0012B6:  66 89 44 20                  mov      dword ptr [si + 0x20], eax
0012BA:  8B 1E 72 06                  mov      bx, word ptr [0x672]
0012BE:  66 26 8B 47 0E               mov      eax, dword ptr es:[bx + 0xe]
0012C3:  8B 1E 70 06                  mov      bx, word ptr [0x670]
0012C7:  66 26 8B 4F 0E               mov      ecx, dword ptr es:[bx + 0xe]
0012CC:  66 2B C1                     sub      eax, ecx
0012CF:  66 F7 EF                     imul     edi
0012D2:  66 0F AC D0 10               shrd     eax, edx, 0x10
0012D7:  66 89 44 28                  mov      dword ptr [si + 0x28], eax
0012DB:  F7 06 1C 06 00 80            test     word ptr [0x61c], 0x8000
0012E1:  C7 44 2C 5E 0D               mov      word ptr [si + 0x2c], 0xd5e
0012E6:  74 41                        je       0x1329
0012E8:  66 0F B7 1E 1C 06            movzx    ebx, word ptr [0x61c]
0012EE:  F7 DB                        neg      bx
0012F0:  29 5C 2E                     sub      word ptr [si + 0x2e], bx
0012F3:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
0012F7:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
0012FB:  66 0F AF C3                  imul     eax, ebx
0012FF:  66 0F AF CB                  imul     ecx, ebx
001303:  66 01 44 08                  add      dword ptr [si + 8], eax
001307:  66 01 4C 18                  add      dword ptr [si + 0x18], ecx
00130B:  66 8B 44 24                  mov      eax, dword ptr [si + 0x24]
00130F:  8B 4C 4A                     mov      cx, word ptr [si + 0x4a]
001312:  8B 54 4C                     mov      dx, word ptr [si + 0x4c]
001315:  66 0F AF C3                  imul     eax, ebx
001319:  0F AF CB                     imul     cx, bx
00131C:  0F AF D3                     imul     dx, bx
00131F:  66 01 44 20                  add      dword ptr [si + 0x20], eax
001323:  01 4C 42                     add      word ptr [si + 0x42], cx
001326:  01 54 44                     add      word ptr [si + 0x44], dx
001329:  8B 04                        mov      ax, word ptr [si]
00132B:  A3 08 09                     mov      word ptr [0x908], ax
00132E:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
001332:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
001336:  BB 64 09                     mov      bx, 0x964
001339:  8B 3F                        mov      di, word ptr [bx]
00133B:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
00133F:  7C 1A                        jl       0x135b
001341:  75 06                        jne      0x1349
001343:  66 3B 4D 0C                  cmp      ecx, dword ptr [di + 0xc]
001347:  7E 12                        jle      0x135b
001349:  8B DF                        mov      bx, di
00134B:  8B 3D                        mov      di, word ptr [di]
00134D:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
001351:  7F F6                        jg       0x1349
001353:  75 06                        jne      0x135b
001355:  66 3B 4D 08                  cmp      ecx, dword ptr [di + 8]
001359:  7F EE                        jg       0x1349
00135B:  89 37                        mov      word ptr [bx], si
00135D:  89 5C 10                     mov      word ptr [si + 0x10], bx
001360:  89 3C                        mov      word ptr [si], di
001362:  89 75 10                     mov      word ptr [di + 0x10], si
001365:  C3                           ret
001366:  C3                           ret
