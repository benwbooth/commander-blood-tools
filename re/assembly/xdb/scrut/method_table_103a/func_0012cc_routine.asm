; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0011a9
; routine_entry: 0x0012cc
; group: method_table_103a
; provenance: alien_method_table_103a_slot_3@0x43f0
; byte_count: 336
; boundary: cfg_blocks_11_terminals_2
; terminal: ret:2
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 20158654ed5aebb114a5588d50d623c6468dbf626533d846f5a47c5746e6c0a5

; -- internal initializer reached only from the method entry at 0x0012cc --
0011A9:  C7 45 36 01 00                mov      word ptr [di + 0x36], 1
0011AE:  2E 8B 2E A3 0D                mov      bp, word ptr cs:[0xda3]
0011B3:  2E C7 06 72 0B 07 00          mov      word ptr cs:[0xb72], 7
0011BA:  66 33 DB                      xor      ebx, ebx
0011BD:  66 33 D2                      xor      edx, edx
0011C0:  66 C7 44 42 A4 06 00 00       mov      dword ptr [si + 0x42], 0x6a4
0011C8:  66 C7 44 46 00 00 00 00       mov      dword ptr [si + 0x46], 0
0011D0:  66 C7 44 4A 00 00 00 00       mov      dword ptr [si + 0x4a], 0
0011D8:  66 B8 A4 06 00 00             mov      eax, 0x6a4
0011DE:  C7 44 0E F9 12                mov      word ptr [si + 0xe], 0x12f9
0011E3:  C7 44 56 19 00                mov      word ptr [si + 0x56], 0x19
0011E8:  C7 44 58 00 00                mov      word ptr [si + 0x58], 0
0011ED:  89 6C 5A                      mov      word ptr [si + 0x5a], bp
0011F0:  C7 44 5C 57 A9                mov      word ptr [si + 0x5c], 0xa957
0011F5:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
0011FA:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
0011FF:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
001204:  C7 44 54 00 00                mov      word ptr [si + 0x54], 0
001209:  2E C7 86 A9 0D 00 00          mov      word ptr cs:[bp + 0xda9], 0
001210:  2E C7 86 AB 0D 00 00          mov      word ptr cs:[bp + 0xdab], 0
001217:  2E C7 86 AD 0D 46 00          mov      word ptr cs:[bp + 0xdad], 0x46
00121E:  2E C7 86 AF 0D 00 00          mov      word ptr cs:[bp + 0xdaf], 0
001225:  49                            dec      cx
001226:  0F 84 95 00                   je       0x12bf
00122A:  83 ED 08                      sub      bp, 8
00122D:  2E FF 06 A1 0D                inc      word ptr cs:[0xda1]
001232:  74 2C                         je       0x1260
001234:  C7 45 36 FF FF                mov      word ptr [di + 0x36], 0xffff
001239:  C7 44 0E 5A 14                mov      word ptr [si + 0xe], 0x145a
00123E:  2E C7 86 AD 0D 00 00          mov      word ptr cs:[bp + 0xdad], 0
001245:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
00124A:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
00124F:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
001254:  66 89 44 42                   mov      dword ptr [si + 0x42], eax
001258:  66 89 5C 46                   mov      dword ptr [si + 0x46], ebx
00125C:  66 89 54 4A                   mov      dword ptr [si + 0x4a], edx
001260:  BF 00 00                      mov      di, 0
001263:  83 C6 5E                      add      si, 0x5e
001266:  83 ED 08                      sub      bp, 8
001269:  81 E5 FF 03                   and      bp, 0x3ff
00126D:  81 C7 00 01                   add      di, 0x100
001271:  C7 44 0E 5A 14                mov      word ptr [si + 0xe], 0x145a
001276:  89 7C 58                      mov      word ptr [si + 0x58], di
001279:  89 6C 5A                      mov      word ptr [si + 0x5a], bp
00127C:  C7 44 5C 00 00                mov      word ptr [si + 0x5c], 0
001281:  2E C7 86 A9 0D 00 00          mov      word ptr cs:[bp + 0xda9], 0
001288:  2E C7 86 AB 0D 00 00          mov      word ptr cs:[bp + 0xdab], 0
00128F:  2E C7 86 AD 0D 00 00          mov      word ptr cs:[bp + 0xdad], 0
001296:  2E C7 86 AF 0D 00 00          mov      word ptr cs:[bp + 0xdaf], 0
00129D:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
0012A2:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
0012A7:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
0012AC:  C7 44 54 00 00                mov      word ptr [si + 0x54], 0
0012B1:  66 89 44 42                   mov      dword ptr [si + 0x42], eax
0012B5:  66 89 5C 46                   mov      dword ptr [si + 0x46], ebx
0012B9:  66 89 54 4A                   mov      dword ptr [si + 0x4a], edx
0012BD:  E2 A4                         loop     0x1263
0012BF:  83 ED 08                      sub      bp, 8
0012C2:  81 E5 FC 03                   and      bp, 0x3fc
0012C6:  2E 89 2E A3 0D                mov      word ptr cs:[0xda3], bp
0012CB:  C3                            ret
; -- method-table entry --
0012CC:  8B 75 16                     mov      si, word ptr [di + 0x16]
0012CF:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
0012D2:  83 C6 5E                     add      si, 0x5e
0012D5:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
0012DA:  0F 84 CB FE                  je       0x11a9
0012DE:  78 0E                        js       0x12ee
0012E0:  2E FF 0E 72 0B               dec      word ptr cs:[0xb72]
0012E5:  79 07                        jns      0x12ee
0012E7:  2E C7 06 72 0B 07 00         mov      word ptr cs:[0xb72], 7
0012EE:  51                           push     cx
0012EF:  FF 54 0E                     call     word ptr [si + 0xe]
0012F2:  59                           pop      cx
0012F3:  83 C6 5E                     add      si, 0x5e
0012F6:  E2 F6                        loop     0x12ee
0012F8:  C3                           ret     
