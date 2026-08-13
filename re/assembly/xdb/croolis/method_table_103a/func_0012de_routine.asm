; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0011bb
; routine_entry: 0x0012de
; group: method_table_103a
; provenance: alien_method_table_103a_slot_3@0x4330
; byte_count: 336
; boundary: cfg_blocks_11_terminals_2
; terminal: ret:2
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: b460ca3c02bd2ce6f46461755f989d0a2914ba888d86c4c665b0f4ad6cfb394b

; -- internal initializer reached only from the method entry at 0x0012de --
0011BB:  C7 45 36 01 00                mov      word ptr [di + 0x36], 1
0011C0:  2E 8B 2E B5 0D                mov      bp, word ptr cs:[0xdb5]
0011C5:  2E C7 06 72 0B 07 00          mov      word ptr cs:[0xb72], 7
0011CC:  66 33 C0                      xor      eax, eax
0011CF:  66 33 D2                      xor      edx, edx
0011D2:  66 C7 44 42 00 00 00 00       mov      dword ptr [si + 0x42], 0
0011DA:  66 C7 44 46 A4 06 00 00       mov      dword ptr [si + 0x46], 0x6a4
0011E2:  66 C7 44 4A 00 00 00 00       mov      dword ptr [si + 0x4a], 0
0011EA:  66 BB A4 06 00 00             mov      ebx, 0x6a4
0011F0:  C7 44 0E 0B 13                mov      word ptr [si + 0xe], 0x130b
0011F5:  C7 44 56 19 00                mov      word ptr [si + 0x56], 0x19
0011FA:  C7 44 58 00 00                mov      word ptr [si + 0x58], 0
0011FF:  89 6C 5A                      mov      word ptr [si + 0x5a], bp
001202:  C7 44 5C 57 A9                mov      word ptr [si + 0x5c], 0xa957
001207:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
00120C:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
001211:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
001216:  C7 44 54 00 00                mov      word ptr [si + 0x54], 0
00121B:  2E C7 86 BB 0D 00 00          mov      word ptr cs:[bp + 0xdbb], 0
001222:  2E C7 86 BD 0D 00 00          mov      word ptr cs:[bp + 0xdbd], 0
001229:  2E C7 86 BF 0D 46 00          mov      word ptr cs:[bp + 0xdbf], 0x46
001230:  2E C7 86 C1 0D 00 00          mov      word ptr cs:[bp + 0xdc1], 0
001237:  49                            dec      cx
001238:  0F 84 95 00                   je       0x12d1
00123C:  83 ED 08                      sub      bp, 8
00123F:  2E FF 06 B3 0D                inc      word ptr cs:[0xdb3]
001244:  74 2C                         je       0x1272
001246:  C7 45 36 FF FF                mov      word ptr [di + 0x36], 0xffff
00124B:  C7 44 0E 6C 14                mov      word ptr [si + 0xe], 0x146c
001250:  2E C7 86 BF 0D 00 00          mov      word ptr cs:[bp + 0xdbf], 0
001257:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
00125C:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
001261:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
001266:  66 89 44 42                   mov      dword ptr [si + 0x42], eax
00126A:  66 89 5C 46                   mov      dword ptr [si + 0x46], ebx
00126E:  66 89 54 4A                   mov      dword ptr [si + 0x4a], edx
001272:  BF 00 00                      mov      di, 0
001275:  83 C6 5E                      add      si, 0x5e
001278:  83 ED 08                      sub      bp, 8
00127B:  81 E5 FF 03                   and      bp, 0x3ff
00127F:  81 C7 00 01                   add      di, 0x100
001283:  C7 44 0E 6C 14                mov      word ptr [si + 0xe], 0x146c
001288:  89 7C 58                      mov      word ptr [si + 0x58], di
00128B:  89 6C 5A                      mov      word ptr [si + 0x5a], bp
00128E:  C7 44 5C 00 00                mov      word ptr [si + 0x5c], 0
001293:  2E C7 86 BB 0D 00 00          mov      word ptr cs:[bp + 0xdbb], 0
00129A:  2E C7 86 BD 0D 00 00          mov      word ptr cs:[bp + 0xdbd], 0
0012A1:  2E C7 86 BF 0D 00 00          mov      word ptr cs:[bp + 0xdbf], 0
0012A8:  2E C7 86 C1 0D 00 00          mov      word ptr cs:[bp + 0xdc1], 0
0012AF:  C7 44 4E 00 00                mov      word ptr [si + 0x4e], 0
0012B4:  C7 44 50 00 00                mov      word ptr [si + 0x50], 0
0012B9:  C7 44 52 00 00                mov      word ptr [si + 0x52], 0
0012BE:  C7 44 54 00 00                mov      word ptr [si + 0x54], 0
0012C3:  66 89 44 42                   mov      dword ptr [si + 0x42], eax
0012C7:  66 89 5C 46                   mov      dword ptr [si + 0x46], ebx
0012CB:  66 89 54 4A                   mov      dword ptr [si + 0x4a], edx
0012CF:  E2 A4                         loop     0x1275
0012D1:  83 ED 08                      sub      bp, 8
0012D4:  81 E5 FC 03                   and      bp, 0x3fc
0012D8:  2E 89 2E B5 0D                mov      word ptr cs:[0xdb5], bp
0012DD:  C3                            ret
; -- method-table entry --
0012DE:  8B 75 16                     mov      si, word ptr [di + 0x16]
0012E1:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
0012E4:  83 C6 5E                     add      si, 0x5e
0012E7:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
0012EC:  0F 84 CB FE                  je       0x11bb
0012F0:  78 0E                        js       0x1300
0012F2:  2E FF 0E 72 0B               dec      word ptr cs:[0xb72]
0012F7:  79 07                        jns      0x1300
0012F9:  2E C7 06 72 0B 07 00         mov      word ptr cs:[0xb72], 7
001300:  51                           push     cx
001301:  FF 54 0E                     call     word ptr [si + 0xe]
001304:  59                           pop      cx
001305:  83 C6 5E                     add      si, 0x5e
001308:  E2 F6                        loop     0x1300
00130A:  C3                           ret     
