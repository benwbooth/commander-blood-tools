; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000223
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 205
; boundary: cfg_blocks_20_terminals_4
; terminal: jmp 0x2be:1, ret:3
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/direct_calls/func_000223_routine.cpp
; routine_bytes_sha256: 66cc8762f57e7fe55e6dd95eb82acb977851b95c46ed0b75a9665a39a8ef9a59

000223:  B8 03 00                     mov      ax, 3
000226:  CD 33                        int      0x33
000228:  81 E9 40 01                  sub      cx, 0x140
00022C:  81 EA 00 02                  sub      dx, 0x200
000230:  89 0E 2A 00                  mov      word ptr [0x2a], cx
000234:  89 16 2C 00                  mov      word ptr [0x2c], dx
000238:  89 1E 2E 00                  mov      word ptr [0x2e], bx
00023C:  D1 F9                        sar      cx, 1
00023E:  83 E9 05                     sub      cx, 5
000241:  79 07                        jns      0x24a
000243:  83 C1 0A                     add      cx, 0xa
000246:  78 02                        js       0x24a
000248:  33 C9                        xor      cx, cx
00024A:  2B 0E 58 10                  sub      cx, word ptr [0x1058]
00024E:  D1 F9                        sar      cx, 1
000250:  89 0E 58 10                  mov      word ptr [0x1058], cx
000254:  01 0E F8 22                  add      word ptr [0x22f8], cx
000258:  C1 E1 03                     shl      cx, 3
00025B:  2B 0E FA 22                  sub      cx, word ptr [0x22fa]
00025F:  D1 F9                        sar      cx, 1
000261:  01 0E FA 22                  add      word ptr [0x22fa], cx
000265:  F7 DA                        neg      dx
000267:  83 EA 05                     sub      dx, 5
00026A:  79 07                        jns      0x273
00026C:  83 C2 0A                     add      dx, 0xa
00026F:  78 02                        js       0x273
000271:  33 D2                        xor      dx, dx
000273:  03 D2                        add      dx, dx
000275:  2B 16 F6 22                  sub      dx, word ptr [0x22f6]
000279:  C1 FA 04                     sar      dx, 4
00027C:  01 16 F6 22                  add      word ptr [0x22f6], dx
000280:  A1 FC 22                     mov      ax, word ptr [0x22fc]
000283:  F7 06 2E 00 01 00            test     word ptr [0x2e], 1
000289:  74 03                        je       0x28e
00028B:  05 0A 00                     add      ax, 0xa
00028E:  F7 06 2E 00 02 00            test     word ptr [0x2e], 2
000294:  74 08                        je       0x29e
000296:  8B D8                        mov      bx, ax
000298:  C1 FB 03                     sar      bx, 3
00029B:  2B C3                        sub      ax, bx
00029D:  48                           dec      ax
00029E:  3D F8 FF                     cmp      ax, 0xfff8
0002A1:  7F 10                        jg       0x2b3
0002A3:  05 08 00                     add      ax, 8
0002A6:  F7 06 82 22 01 00            test     word ptr [0x2282], 1
0002AC:  74 10                        je       0x2be
0002AE:  2D 40 00                     sub      ax, 0x40
0002B1:  EB 0B                        jmp      0x2be
0002B3:  F7 06 82 22 01 00            test     word ptr [0x2282], 1
0002B9:  74 03                        je       0x2be
0002BB:  B8 9C FF                     mov      ax, 0xff9c
0002BE:  A3 FC 22                     mov      word ptr [0x22fc], ax
0002C1:  C7 06 82 22 00 00            mov      word ptr [0x2282], 0
0002C7:  2E A1 95 00                  mov      ax, word ptr cs:[0x95]
0002CB:  3D 00 48                     cmp      ax, 0x4800
0002CE:  74 13                        je       0x2e3
0002D0:  3D 00 50                     cmp      ax, 0x5000
0002D3:  74 01                        je       0x2d6
0002D5:  C3                           ret     
0002D6:  2E C7 06 95 00 00 00         mov      word ptr cs:[0x95], 0
0002DD:  83 2E FC 22 08               sub      word ptr [0x22fc], 8
0002E2:  C3                           ret     
0002E3:  2E C7 06 95 00 00 00         mov      word ptr cs:[0x95], 0
0002EA:  83 06 FC 22 08               add      word ptr [0x22fc], 8
0002EF:  C3                           ret     
