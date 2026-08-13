; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001163
; routine_entry: 0x001286
; group: method_table_103a
; provenance: alien_method_table_103a_slot_3@0x42c0
; byte_count: 336
; boundary: cfg_blocks_11_terminals_2
; terminal: ret:2
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 69eb44d3314aa67fcb427aa63c764d3b67ca4286e5ffad553b7488605b1d594b

; -- internal initializer reached only from the method entry at 0x001286 --
001163:  C7 45 36 01 00                mov      word ptr [di + 0x36], 1
001168:  2E 8B 2E 5D 0D                mov      bp, word ptr cs:[0xd5d]
00116D:  2E C7 06 31 0B 07 00          mov      word ptr cs:[0xb31], 7
001174:  66 33 C0                      xor      eax, eax
001177:  66 33 D2                      xor      edx, edx
00117A:  66 C7 44 42 00 00 00 00       mov      dword ptr [si + 0x42], 0
001182:  66 C7 44 46 A4 06 00 00       mov      dword ptr [si + 0x46], 0x6a4
00118A:  66 C7 44 4A 00 00 00 00       mov      dword ptr [si + 0x4a], 0
001192:  66 BB A4 06 00 00             mov      ebx, 0x6a4
001198:  C7 44 0E B3 12                mov      word ptr [si + 0xe], 0x12b3
00119D:  C7 44 56 19 00                mov      word ptr [si + 0x56], 0x19
0011A2:  C7 44 58 00 00                mov      word ptr [si + 0x58], 0
0011A7:  89 6C 5A                      mov      word ptr [si + 0x5a], bp
0011AA:  C7 44 5C 57 A9                mov      word ptr [si + 0x5c], 0xa957
0011AF:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
0011B4:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
0011B9:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
0011BE:  C7 44 54 00 00                mov      word ptr [si + 0x54], 0
0011C3:  2E C7 86 63 0D 00 00          mov      word ptr cs:[bp + 0xd63], 0
0011CA:  2E C7 86 65 0D 00 00          mov      word ptr cs:[bp + 0xd65], 0
0011D1:  2E C7 86 67 0D 46 00          mov      word ptr cs:[bp + 0xd67], 0x46
0011D8:  2E C7 86 69 0D 00 00          mov      word ptr cs:[bp + 0xd69], 0
0011DF:  49                            dec      cx
0011E0:  0F 84 95 00                   je       0x1279
0011E4:  83 ED 08                      sub      bp, 8
0011E7:  2E FF 06 5B 0D                inc      word ptr cs:[0xd5b]
0011EC:  74 2C                         je       0x121a
0011EE:  C7 45 36 FF FF                mov      word ptr [di + 0x36], 0xffff
0011F3:  C7 44 0E 14 14                mov      word ptr [si + 0xe], 0x1414
0011F8:  2E C7 86 67 0D 00 00          mov      word ptr cs:[bp + 0xd67], 0
0011FF:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
001204:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
001209:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
00120E:  66 89 44 42                   mov      dword ptr [si + 0x42], eax
001212:  66 89 5C 46                   mov      dword ptr [si + 0x46], ebx
001216:  66 89 54 4A                   mov      dword ptr [si + 0x4a], edx
00121A:  BF 00 00                      mov      di, 0
00121D:  83 C6 5E                      add      si, 0x5e
001220:  83 ED 08                      sub      bp, 8
001223:  81 E5 FF 03                   and      bp, 0x3ff
001227:  81 C7 00 01                   add      di, 0x100
00122B:  C7 44 0E 14 14                mov      word ptr [si + 0xe], 0x1414
001230:  89 7C 58                      mov      word ptr [si + 0x58], di
001233:  89 6C 5A                      mov      word ptr [si + 0x5a], bp
001236:  C7 44 5C 00 00                mov      word ptr [si + 0x5c], 0
00123B:  2E C7 86 63 0D 00 00          mov      word ptr cs:[bp + 0xd63], 0
001242:  2E C7 86 65 0D 00 00          mov      word ptr cs:[bp + 0xd65], 0
001249:  2E C7 86 67 0D 00 00          mov      word ptr cs:[bp + 0xd67], 0
001250:  2E C7 86 69 0D 00 00          mov      word ptr cs:[bp + 0xd69], 0
001257:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
00125C:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
001261:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
001266:  C7 44 54 00 00                mov      word ptr [si + 0x54], 0
00126B:  66 89 44 42                   mov      dword ptr [si + 0x42], eax
00126F:  66 89 5C 46                   mov      dword ptr [si + 0x46], ebx
001273:  66 89 54 4A                   mov      dword ptr [si + 0x4a], edx
001277:  E2 A4                         loop     0x121d
001279:  83 ED 08                      sub      bp, 8
00127C:  81 E5 FC 03                   and      bp, 0x3fc
001280:  2E 89 2E 5D 0D                mov      word ptr cs:[0xd5d], bp
001285:  C3                            ret
; -- method-table entry --
001286:  8B 75 16                     mov      si, word ptr [di + 0x16]
001289:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
00128C:  83 C6 5E                     add      si, 0x5e
00128F:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
001294:  0F 84 CB FE                  je       0x1163
001298:  78 0E                        js       0x12a8
00129A:  2E FF 0E 31 0B               dec      word ptr cs:[0xb31]
00129F:  79 07                        jns      0x12a8
0012A1:  2E C7 06 31 0B 07 00         mov      word ptr cs:[0xb31], 7
0012A8:  51                           push     cx
0012A9:  FF 54 0E                     call     word ptr [si + 0xe]
0012AC:  59                           pop      cx
0012AD:  83 C6 5E                     add      si, 0x5e
0012B0:  E2 F6                        loop     0x12a8
0012B2:  C3                           ret     
