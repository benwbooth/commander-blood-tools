; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x00022a
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 219
; boundary: cfg_blocks_22_terminals_5
; terminal: jmp 0x2c5:1, ret:4
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 33edea2995d9e02b635337e879349eb351b468967e0d54a930b662861ddeb375

00022A:  B8 03 00                     mov      ax, 3
00022D:  CD 33                        int      0x33
00022F:  81 E9 40 01                  sub      cx, 0x140
000233:  81 EA 00 02                  sub      dx, 0x200
000237:  89 0E 2A 00                  mov      word ptr [0x2a], cx
00023B:  89 16 2C 00                  mov      word ptr [0x2c], dx
00023F:  89 1E 2E 00                  mov      word ptr [0x2e], bx
000243:  D1 F9                        sar      cx, 1
000245:  83 E9 05                     sub      cx, 5
000248:  79 07                        jns      0x251
00024A:  83 C1 0A                     add      cx, 0xa
00024D:  78 02                        js       0x251
00024F:  33 C9                        xor      cx, cx
000251:  2B 0E 58 10                  sub      cx, word ptr [0x1058]
000255:  D1 F9                        sar      cx, 1
000257:  89 0E 58 10                  mov      word ptr [0x1058], cx
00025B:  01 0E F8 22                  add      word ptr [0x22f8], cx
00025F:  C1 E1 03                     shl      cx, 3
000262:  2B 0E FA 22                  sub      cx, word ptr [0x22fa]
000266:  D1 F9                        sar      cx, 1
000268:  01 0E FA 22                  add      word ptr [0x22fa], cx
00026C:  F7 DA                        neg      dx
00026E:  83 EA 05                     sub      dx, 5
000271:  79 07                        jns      0x27a
000273:  83 C2 0A                     add      dx, 0xa
000276:  78 02                        js       0x27a
000278:  33 D2                        xor      dx, dx
00027A:  03 D2                        add      dx, dx
00027C:  2B 16 F6 22                  sub      dx, word ptr [0x22f6]
000280:  C1 FA 04                     sar      dx, 4
000283:  01 16 F6 22                  add      word ptr [0x22f6], dx
000287:  A1 FC 22                     mov      ax, word ptr [0x22fc]
00028A:  F7 06 2E 00 01 00            test     word ptr [0x2e], 1
000290:  74 03                        je       0x295
000292:  05 0A 00                     add      ax, 0xa
000295:  F7 06 2E 00 02 00            test     word ptr [0x2e], 2
00029B:  74 08                        je       0x2a5
00029D:  8B D8                        mov      bx, ax
00029F:  C1 FB 03                     sar      bx, 3
0002A2:  2B C3                        sub      ax, bx
0002A4:  48                           dec      ax
0002A5:  3D F8 FF                     cmp      ax, 0xfff8
0002A8:  7F 10                        jg       0x2ba
0002AA:  05 08 00                     add      ax, 8
0002AD:  F7 06 82 22 FF FF            test     word ptr [0x2282], 0xffff
0002B3:  74 10                        je       0x2c5
0002B5:  2D 40 00                     sub      ax, 0x40
0002B8:  EB 0B                        jmp      0x2c5
0002BA:  F7 06 82 22 FF FF            test     word ptr [0x2282], 0xffff
0002C0:  74 03                        je       0x2c5
0002C2:  B8 9C FF                     mov      ax, 0xff9c
0002C5:  A3 FC 22                     mov      word ptr [0x22fc], ax
0002C8:  2E A1 95 00                  mov      ax, word ptr cs:[0x95]
0002CC:  2E C7 06 95 00 00 00         mov      word ptr cs:[0x95], 0
0002D3:  3D 00 48                     cmp      ax, 0x4800
0002D6:  74 17                        je       0x2ef
0002D8:  3D 00 50                     cmp      ax, 0x5000
0002DB:  74 05                        je       0x2e2
0002DD:  3C 20                        cmp      al, 0x20
0002DF:  74 1D                        je       0x2fe
0002E1:  C3                           ret     
0002E2:  2E C7 06 95 00 00 00         mov      word ptr cs:[0x95], 0
0002E9:  83 2E FC 22 08               sub      word ptr [0x22fc], 8
0002EE:  C3                           ret     
0002EF:  2E C7 06 95 00 00 00         mov      word ptr cs:[0x95], 0
0002F6:  83 06 FC 22 08               add      word ptr [0x22fc], 8
0002FB:  C3                           ret     
; -- non-contiguous block: next 0x0002fe --
0002FE:  2E 83 0E FC 02 10            or       word ptr cs:[0x2fc], 0x10
000304:  C3                           ret     
