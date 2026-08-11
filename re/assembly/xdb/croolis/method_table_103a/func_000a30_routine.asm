; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000a30
; group: method_table_103a
; provenance: alien_method_table_103a_slot_1@0x432c
; byte_count: 288
; boundary: cfg_blocks_15_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 0eb73213de1c93215d04029e4e35a966aebab191513a24408512a6145e17e822

000A30:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
000A35:  74 BE                        je       0x9f5
000A37:  8B 75 16                     mov      si, word ptr [di + 0x16]
000A3A:  83 C6 5E                     add      si, 0x5e
000A3D:  FF 44 50                     inc      word ptr [si + 0x50]
000A40:  8B 6D 38                     mov      bp, word ptr [di + 0x38]
000A43:  81 E5 FC 0F                  and      bp, 0xffc
000A47:  64 8B 86 36 00               mov      ax, word ptr fs:[bp + 0x36]
000A4C:  C1 F8 08                     sar      ax, 8
000A4F:  2E A3 76 0B                  mov      word ptr cs:[0xb76], ax
000A53:  2E F7 06 70 0B 01 00         test     word ptr cs:[0xb70], 1
000A5A:  74 44                        je       0xaa0
000A5C:  2D 3C 00                     sub      ax, 0x3c
000A5F:  03 44 46                     add      ax, word ptr [si + 0x46]
000A62:  03 06 F0 22                  add      ax, word ptr [0x22f0]
000A66:  78 38                        js       0xaa0
000A68:  3D 80 00                     cmp      ax, 0x80
000A6B:  7F 33                        jg       0xaa0
000A6D:  8B 44 42                     mov      ax, word ptr [si + 0x42]
000A70:  03 06 EC 22                  add      ax, word ptr [0x22ec]
000A74:  3D 00 FF                     cmp      ax, 0xff00
000A77:  7C 27                        jl       0xaa0
000A79:  3D 00 01                     cmp      ax, 0x100
000A7C:  7F 22                        jg       0xaa0
000A7E:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
000A81:  03 06 F4 22                  add      ax, word ptr [0x22f4]
000A85:  3D 00 FF                     cmp      ax, 0xff00
000A88:  7C 16                        jl       0xaa0
000A8A:  3D 00 01                     cmp      ax, 0x100
000A8D:  7F 11                        jg       0xaa0
000A8F:  2E C7 06 70 0B 02 00         mov      word ptr cs:[0xb70], 2
000A96:  2E 89 36 74 0B               mov      word ptr cs:[0xb74], si
000A9B:  C7 45 3A 70 01               mov      word ptr [di + 0x3a], 0x170
000AA0:  1E                           push     ds
000AA1:  8B 55 3A                     mov      dx, word ptr [di + 0x3a]
000AA4:  83 FA 30                     cmp      dx, 0x30
000AA7:  7E 06                        jle      0xaaf
000AA9:  83 EA 04                     sub      dx, 4
000AAC:  89 55 3A                     mov      word ptr [di + 0x3a], dx
000AAF:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
000AB4:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
000AB8:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
000ABC:  64 8B 6D 38                  mov      bp, word ptr fs:[di + 0x38]
000AC0:  64 01 55 38                  add      word ptr fs:[di + 0x38], dx
000AC4:  8B 5C 08                     mov      bx, word ptr [si + 8]
000AC7:  03 DB                        add      bx, bx
000AC9:  03 DD                        add      bx, bp
000ACB:  81 E3 FC 0F                  and      bx, 0xffc
000ACF:  64 8B 87 36 00               mov      ax, word ptr fs:[bx + 0x36]
000AD4:  C1 F8 08                     sar      ax, 8
000AD7:  29 44 06                     sub      word ptr [si + 6], ax
000ADA:  03 DA                        add      bx, dx
000ADC:  81 E3 FC 0F                  and      bx, 0xffc
000AE0:  64 8B 87 36 00               mov      ax, word ptr fs:[bx + 0x36]
000AE5:  C1 F8 08                     sar      ax, 8
000AE8:  01 44 06                     add      word ptr [si + 6], ax
000AEB:  83 C6 14                     add      si, 0x14
000AEE:  E2 D4                        loop     0xac4
000AF0:  64 8B 55 3E                  mov      dx, word ptr fs:[di + 0x3e]
000AF4:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
000AF8:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
000AFC:  64 01 55 3C                  add      word ptr fs:[di + 0x3c], dx
000B00:  64 8B 7D 3C                  mov      di, word ptr fs:[di + 0x3c]
000B04:  8B 5C 04                     mov      bx, word ptr [si + 4]
000B07:  83 EB 19                     sub      bx, 0x19
000B0A:  79 07                        jns      0xb13
000B0C:  F7 DB                        neg      bx
000B0E:  83 EB 32                     sub      bx, 0x32
000B11:  78 36                        js       0xb49
000B13:  03 DB                        add      bx, bx
000B15:  66 0F B7 EB                  movzx    ebp, bx
000B19:  03 DF                        add      bx, di
000B1B:  81 E3 FC 0F                  and      bx, 0xffc
000B1F:  66 64 0F BF 87 36 00         movsx    eax, word ptr fs:[bx + 0x36]
000B26:  66 0F AF C5                  imul     eax, ebp
000B2A:  66 C1 F8 11                  sar      eax, 0x11
000B2E:  29 44 06                     sub      word ptr [si + 6], ax
000B31:  03 DA                        add      bx, dx
000B33:  81 E3 FC 0F                  and      bx, 0xffc
000B37:  66 64 0F BF 87 36 00         movsx    eax, word ptr fs:[bx + 0x36]
000B3E:  66 0F AF C5                  imul     eax, ebp
000B42:  66 C1 F8 11                  sar      eax, 0x11
000B46:  01 44 06                     add      word ptr [si + 6], ax
000B49:  83 C6 14                     add      si, 0x14
000B4C:  E2 B6                        loop     0xb04
000B4E:  1F                           pop      ds
000B4F:  C3                           ret     
