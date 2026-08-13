; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0009f5
; routine_entry: 0x000a35
; group: method_table_103a
; provenance: alien_method_table_103a_slot_1@0x43ec
; byte_count: 352
; boundary: cfg_blocks_16_terminals_2
; terminal: ret:2
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 7bc6cebc553f911f03f8aecabe977e7abee802da76edf969433c7c3995a01946

; -- internal initializer reached only from the method entry at 0x000a35 --
0009F5:  C7 45 36 01 00               mov      word ptr [di + 0x36], 1
0009FA:  2E C7 06 70 0B 00 00         mov      word ptr cs:[0xb70], 0
000A01:  8B 75 16                     mov      si, word ptr [di + 0x16]
000A04:  83 C6 5E                     add      si, 0x5e
000A07:  C7 45 38 04 00               mov      word ptr [di + 0x38], 4
000A0C:  C7 45 3A 30 00               mov      word ptr [di + 0x3a], 0x30
000A11:  C7 45 3C 04 00               mov      word ptr [di + 0x3c], 4
000A16:  C7 45 3E 10 00               mov      word ptr [di + 0x3e], 0x10
000A1B:  C7 44 54 0C 00               mov      word ptr [si + 0x54], 0xc
000A20:  C7 44 4E 00 00               mov      word ptr [si + 0x4e], 0
000A25:  C7 44 50 00 00               mov      word ptr [si + 0x50], 0
000A2A:  C7 44 52 00 00               mov      word ptr [si + 0x52], 0
000A2F:  2E 89 36 74 0B               mov      word ptr cs:[0xb74], si
000A34:  C3                           ret
; -- method-table entry --
000A35:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
000A3A:  74 B9                        je       0x9f5
000A3C:  8B 75 16                     mov      si, word ptr [di + 0x16]
000A3F:  83 C6 5E                     add      si, 0x5e
000A42:  FF 44 50                     inc      word ptr [si + 0x50]
000A45:  8B 6D 38                     mov      bp, word ptr [di + 0x38]
000A48:  81 E5 FC 0F                  and      bp, 0xffc
000A4C:  64 8B 86 36 00               mov      ax, word ptr fs:[bp + 0x36]
000A51:  C1 F8 08                     sar      ax, 8
000A54:  2E A3 76 0B                  mov      word ptr cs:[0xb76], ax
000A58:  2E F7 06 70 0B 01 00         test     word ptr cs:[0xb70], 1
000A5F:  74 44                        je       0xaa5
000A61:  2D 3C 00                     sub      ax, 0x3c
000A64:  03 44 46                     add      ax, word ptr [si + 0x46]
000A67:  03 06 F0 22                  add      ax, word ptr [0x22f0]
000A6B:  78 38                        js       0xaa5
000A6D:  3D 80 00                     cmp      ax, 0x80
000A70:  7F 33                        jg       0xaa5
000A72:  8B 44 42                     mov      ax, word ptr [si + 0x42]
000A75:  03 06 EC 22                  add      ax, word ptr [0x22ec]
000A79:  3D 00 FF                     cmp      ax, 0xff00
000A7C:  7C 27                        jl       0xaa5
000A7E:  3D 00 01                     cmp      ax, 0x100
000A81:  7F 22                        jg       0xaa5
000A83:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
000A86:  03 06 F4 22                  add      ax, word ptr [0x22f4]
000A8A:  3D 00 FF                     cmp      ax, 0xff00
000A8D:  7C 16                        jl       0xaa5
000A8F:  3D 00 01                     cmp      ax, 0x100
000A92:  7F 11                        jg       0xaa5
000A94:  2E C7 06 70 0B 02 00         mov      word ptr cs:[0xb70], 2
000A9B:  2E 89 36 74 0B               mov      word ptr cs:[0xb74], si
000AA0:  C7 45 3A 70 01               mov      word ptr [di + 0x3a], 0x170
000AA5:  1E                           push     ds
000AA6:  8B 55 3A                     mov      dx, word ptr [di + 0x3a]
000AA9:  83 FA 30                     cmp      dx, 0x30
000AAC:  7E 06                        jle      0xab4
000AAE:  83 EA 04                     sub      dx, 4
000AB1:  89 55 3A                     mov      word ptr [di + 0x3a], dx
000AB4:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
000AB9:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
000ABD:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
000AC1:  64 8B 6D 38                  mov      bp, word ptr fs:[di + 0x38]
000AC5:  64 01 55 38                  add      word ptr fs:[di + 0x38], dx
000AC9:  8B 5C 08                     mov      bx, word ptr [si + 8]
000ACC:  03 DB                        add      bx, bx
000ACE:  03 DD                        add      bx, bp
000AD0:  81 E3 FC 0F                  and      bx, 0xffc
000AD4:  64 8B 87 36 00               mov      ax, word ptr fs:[bx + 0x36]
000AD9:  C1 F8 08                     sar      ax, 8
000ADC:  29 44 06                     sub      word ptr [si + 6], ax
000ADF:  03 DA                        add      bx, dx
000AE1:  81 E3 FC 0F                  and      bx, 0xffc
000AE5:  64 8B 87 36 00               mov      ax, word ptr fs:[bx + 0x36]
000AEA:  C1 F8 08                     sar      ax, 8
000AED:  01 44 06                     add      word ptr [si + 6], ax
000AF0:  83 C6 14                     add      si, 0x14
000AF3:  E2 D4                        loop     0xac9
000AF5:  64 8B 55 3E                  mov      dx, word ptr fs:[di + 0x3e]
000AF9:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
000AFD:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
000B01:  64 01 55 3C                  add      word ptr fs:[di + 0x3c], dx
000B05:  64 8B 7D 3C                  mov      di, word ptr fs:[di + 0x3c]
000B09:  8B 5C 04                     mov      bx, word ptr [si + 4]
000B0C:  83 EB 19                     sub      bx, 0x19
000B0F:  79 07                        jns      0xb18
000B11:  F7 DB                        neg      bx
000B13:  83 EB 32                     sub      bx, 0x32
000B16:  78 36                        js       0xb4e
000B18:  03 DB                        add      bx, bx
000B1A:  66 0F B7 EB                  movzx    ebp, bx
000B1E:  03 DF                        add      bx, di
000B20:  81 E3 FC 0F                  and      bx, 0xffc
000B24:  66 64 0F BF 87 36 00         movsx    eax, word ptr fs:[bx + 0x36]
000B2B:  66 0F AF C5                  imul     eax, ebp
000B2F:  66 C1 F8 11                  sar      eax, 0x11
000B33:  29 44 06                     sub      word ptr [si + 6], ax
000B36:  03 DA                        add      bx, dx
000B38:  81 E3 FC 0F                  and      bx, 0xffc
000B3C:  66 64 0F BF 87 36 00         movsx    eax, word ptr fs:[bx + 0x36]
000B43:  66 0F AF C5                  imul     eax, ebp
000B47:  66 C1 F8 11                  sar      eax, 0x11
000B4B:  01 44 06                     add      word ptr [si + 6], ax
000B4E:  83 C6 14                     add      si, 0x14
000B51:  E2 B6                        loop     0xb09
000B53:  1F                           pop      ds
000B54:  C3                           ret     
