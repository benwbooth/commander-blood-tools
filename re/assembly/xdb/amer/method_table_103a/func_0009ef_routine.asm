; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x0009ef
; group: method_table_103a
; provenance: alien_method_table_103a_slot_1@0x42bc
; byte_count: 288
; boundary: cfg_blocks_15_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/method_table_103a/func_0009ef_routine.cpp
; routine_bytes_sha256: c0951844b3334d3b24815a6347e8051452aa4472ed0fe310563d0cb649ac5f3c

0009EF:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
0009F4:  74 BE                        je       0x9b4
0009F6:  8B 75 16                     mov      si, word ptr [di + 0x16]
0009F9:  83 C6 5E                     add      si, 0x5e
0009FC:  FF 44 50                     inc      word ptr [si + 0x50]
0009FF:  8B 6D 38                     mov      bp, word ptr [di + 0x38]
000A02:  81 E5 FC 0F                  and      bp, 0xffc
000A06:  64 8B 86 36 00               mov      ax, word ptr fs:[bp + 0x36]
000A0B:  C1 F8 08                     sar      ax, 8
000A0E:  2E A3 35 0B                  mov      word ptr cs:[0xb35], ax
000A12:  2E F7 06 2F 0B 01 00         test     word ptr cs:[0xb2f], 1
000A19:  74 44                        je       0xa5f
000A1B:  2D 3C 00                     sub      ax, 0x3c
000A1E:  03 44 46                     add      ax, word ptr [si + 0x46]
000A21:  03 06 F0 22                  add      ax, word ptr [0x22f0]
000A25:  78 38                        js       0xa5f
000A27:  3D 80 00                     cmp      ax, 0x80
000A2A:  7F 33                        jg       0xa5f
000A2C:  8B 44 42                     mov      ax, word ptr [si + 0x42]
000A2F:  03 06 EC 22                  add      ax, word ptr [0x22ec]
000A33:  3D 00 FF                     cmp      ax, 0xff00
000A36:  7C 27                        jl       0xa5f
000A38:  3D 00 01                     cmp      ax, 0x100
000A3B:  7F 22                        jg       0xa5f
000A3D:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
000A40:  03 06 F4 22                  add      ax, word ptr [0x22f4]
000A44:  3D 00 FF                     cmp      ax, 0xff00
000A47:  7C 16                        jl       0xa5f
000A49:  3D 00 01                     cmp      ax, 0x100
000A4C:  7F 11                        jg       0xa5f
000A4E:  2E C7 06 2F 0B 02 00         mov      word ptr cs:[0xb2f], 2
000A55:  2E 89 36 33 0B               mov      word ptr cs:[0xb33], si
000A5A:  C7 45 3A 70 01               mov      word ptr [di + 0x3a], 0x170
000A5F:  1E                           push     ds
000A60:  8B 55 3A                     mov      dx, word ptr [di + 0x3a]
000A63:  83 FA 30                     cmp      dx, 0x30
000A66:  7E 06                        jle      0xa6e
000A68:  83 EA 04                     sub      dx, 4
000A6B:  89 55 3A                     mov      word ptr [di + 0x3a], dx
000A6E:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
000A73:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
000A77:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
000A7B:  64 8B 6D 38                  mov      bp, word ptr fs:[di + 0x38]
000A7F:  64 01 55 38                  add      word ptr fs:[di + 0x38], dx
000A83:  8B 5C 08                     mov      bx, word ptr [si + 8]
000A86:  03 DB                        add      bx, bx
000A88:  03 DD                        add      bx, bp
000A8A:  81 E3 FC 0F                  and      bx, 0xffc
000A8E:  64 8B 87 36 00               mov      ax, word ptr fs:[bx + 0x36]
000A93:  C1 F8 08                     sar      ax, 8
000A96:  29 44 06                     sub      word ptr [si + 6], ax
000A99:  03 DA                        add      bx, dx
000A9B:  81 E3 FC 0F                  and      bx, 0xffc
000A9F:  64 8B 87 36 00               mov      ax, word ptr fs:[bx + 0x36]
000AA4:  C1 F8 08                     sar      ax, 8
000AA7:  01 44 06                     add      word ptr [si + 6], ax
000AAA:  83 C6 14                     add      si, 0x14
000AAD:  E2 D4                        loop     0xa83
000AAF:  64 8B 55 3E                  mov      dx, word ptr fs:[di + 0x3e]
000AB3:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
000AB7:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
000ABB:  64 01 55 3C                  add      word ptr fs:[di + 0x3c], dx
000ABF:  64 8B 7D 3C                  mov      di, word ptr fs:[di + 0x3c]
000AC3:  8B 5C 04                     mov      bx, word ptr [si + 4]
000AC6:  83 EB 19                     sub      bx, 0x19
000AC9:  79 07                        jns      0xad2
000ACB:  F7 DB                        neg      bx
000ACD:  83 EB 32                     sub      bx, 0x32
000AD0:  78 36                        js       0xb08
000AD2:  03 DB                        add      bx, bx
000AD4:  66 0F B7 EB                  movzx    ebp, bx
000AD8:  03 DF                        add      bx, di
000ADA:  81 E3 FC 0F                  and      bx, 0xffc
000ADE:  66 64 0F BF 87 36 00         movsx    eax, word ptr fs:[bx + 0x36]
000AE5:  66 0F AF C5                  imul     eax, ebp
000AE9:  66 C1 F8 11                  sar      eax, 0x11
000AED:  29 44 06                     sub      word ptr [si + 6], ax
000AF0:  03 DA                        add      bx, dx
000AF2:  81 E3 FC 0F                  and      bx, 0xffc
000AF6:  66 64 0F BF 87 36 00         movsx    eax, word ptr fs:[bx + 0x36]
000AFD:  66 0F AF C5                  imul     eax, ebp
000B01:  66 C1 F8 11                  sar      eax, 0x11
000B05:  01 44 06                     add      word ptr [si + 6], ax
000B08:  83 C6 14                     add      si, 0x14
000B0B:  E2 B6                        loop     0xac3
000B0D:  1F                           pop      ds
000B0E:  C3                           ret     
