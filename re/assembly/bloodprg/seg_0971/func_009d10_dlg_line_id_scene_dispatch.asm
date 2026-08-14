; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009d10
; seg_off: 0971:0000
; group: seg_0971
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: dlg_line_id_scene_dispatch
; label_comment: SEG 0x971:0. Consumer of the active line id gs:0x6788 (called from 0x1EDD with ax=[0x6788]). Re-reads it at 0x9D1D, rejects negative (js), branches on [0x1FB2]&1 and on specific ids (cmp ax,0x1D at 0x9D2F). The work it does is GRAPHICS, not audio: at 0x9D80..0x9DAA it raises the palette-refresh gates [0x5B53]/[0x5B57] and [0xAE1], loads the buffer far-ptr [0x5229], and calls 0x1ce:0x91d (unpack), then lowers them. TRACED 2026-07-24 with NO sound call on this path || ALSO RECORDED as `render_dispatch_6788`: render dispatch: [0xdb8]=0; ax=[0x6788]; if negative skip; test [0x1fb2],1. Core per-frame render dispatch keyed on the 0x6788 object/scene index (called 8x) || MERGED 2026-07-25 (#186): one address, several names, folded by union. Natural C and direct vectors: re/source/bloodprg/candidates/seg_0971/func_009d10_dlg_line_id_scene_dispatch.c and re/tools/oracle_vectors/func_9d10_natural.json
; incoming: call@0x0018a5->0971:0000
; incoming: call@0x001ee0->0971:0000
; incoming: call@0x001f53->0971:0000
; incoming: call@0x0077fa->0971:0000
; incoming: call@0x007b74->0971:0000
; incoming: call@0x008b2f->0971:0000
; incoming: call@0x00aff1->0971:0000
; incoming: call@0x00b136->0971:0000
; byte_count: 579
; boundary: cfg_blocks_49_terminals_8
; terminal: jmp 0x9d5c:1, jmp 0x9dbf:1, jmp 0x9e57:1, jmp 0x9f4a:4, retf:1
; direct_callees: 0x00a15f, 0x00a1b4, 0x00a40b
; indirect_calls: 4
; routine_bytes_sha256: f29d509a5222b601da2df320823ffeccac6ae4231ab2e16cffe6a9892e791ae2

009D10:  50                           push     ax
009D11:  53                           push     bx
009D12:  51                           push     cx
009D13:  52                           push     dx
009D14:  57                           push     di
009D15:  56                           push     si
009D16:  06                           push     es
009D17:  55                           push     bp
009D18:  C6 06 B8 0D 00               mov      byte ptr [0xdb8], 0
009D1D:  A1 88 67                     mov      ax, word ptr [0x6788]
009D20:  0B C0                        or       ax, ax
009D22:  0F 88 24 02                  js       0x9f4a
009D26:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
009D2B:  0F 85 74 01                  jne      0x9ea3
009D2F:  83 F8 1D                     cmp      ax, 0x1d
009D32:  75 19                        jne      0x9d4d
009D34:  06                           push     es
009D35:  8E 06 26 67                  mov      es, word ptr [0x6726]
009D39:  8B 3E 5E 67                  mov      di, word ptr [0x675e]
009D3D:  26 8B 5D 02                  mov      bx, word ptr es:[di + 2]
009D41:  07                           pop      es
009D42:  3B 1E 60 67                  cmp      bx, word ptr [0x6760]
009D46:  0F 94 06 E3 0A               sete     byte ptr [0xae3]
009D4B:  EB 0F                        jmp      0x9d5c
009D4D:  F6 06 E3 0A 01               test     byte ptr [0xae3], 1
009D52:  74 08                        je       0x9d5c
009D54:  C6 06 E4 0A 01               mov      byte ptr [0xae4], 1
009D59:  E9 EE 01                     jmp      0x9f4a
009D5C:  F6 06 4F 27 01               test     byte ptr [0x274f], 1
009D61:  0F 85 82 00                  jne      0x9de7
009D65:  8B D8                        mov      bx, ax
009D67:  C1 E3 02                     shl      bx, 2
009D6A:  81 C3 B5 1F                  add      bx, 0x1fb5
009D6E:  8B 77 02                     mov      si, word ptr [bx + 2]
009D71:  83 FE FF                     cmp      si, -1
009D74:  74 43                        je       0x9db9
009D76:  3B 36 A3 1F                  cmp      si, word ptr [0x1fa3]
009D7A:  74 43                        je       0x9dbf
009D7C:  89 36 A3 1F                  mov      word ptr [0x1fa3], si
009D80:  06                           push     es
009D81:  50                           push     ax
009D82:  C6 06 53 5B 01               mov      byte ptr [0x5b53], 1
009D87:  C6 06 57 5B 01               mov      byte ptr [0x5b57], 1
009D8C:  C4 3E 29 52                  les      di, ptr [0x5229]
009D90:  C6 06 E1 0A 01               mov      byte ptr [0xae1], 1
009D95:  9A 1D 09 CE 01               lcall    0x1ce, 0x91d
009D9A:  C6 06 E1 0A 00               mov      byte ptr [0xae1], 0
009D9F:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
009DA4:  C6 06 57 5B 00               mov      byte ptr [0x5b57], 0
009DA9:  58                           pop      ax
009DAA:  07                           pop      es
009DAB:  BE D1 53                     mov      si, 0x53d1
009DAE:  BF D1 59                     mov      di, 0x59d1
009DB1:  B9 30 00                     mov      cx, 0x30
009DB4:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
009DB7:  EB 06                        jmp      0x9dbf
009DB9:  C7 06 A3 1F FF FF            mov      word ptr [0x1fa3], 0xffff
009DBF:  83 3E A3 1F FF               cmp      word ptr [0x1fa3], -1
009DC4:  75 21                        jne      0x9de7
009DC6:  50                           push     ax
009DC7:  A1 A7 1F                     mov      ax, word ptr [0x1fa7]
009DCA:  A3 39 52                     mov      word ptr [0x5239], ax
009DCD:  05 82 00                     add      ax, 0x82
009DD0:  A3 3B 52                     mov      word ptr [0x523b], ax
009DD3:  33 C0                        xor      ax, ax
009DD5:  9A 2F 0E 99 02               lcall    0x299, 0xe2f
009DDA:  C7 06 39 52 00 00            mov      word ptr [0x5239], 0
009DE0:  C7 06 3B 52 C8 00            mov      word ptr [0x523b], 0xc8
009DE6:  58                           pop      ax
009DE7:  C6 06 B2 1F 01               mov      byte ptr [0x1fb2], 1
009DEC:  C6 06 B9 0D 00               mov      byte ptr [0xdb9], 0
009DF1:  C6 06 BB 0D 00               mov      byte ptr [0xdbb], 0
009DF6:  C6 06 BD 0D 00               mov      byte ptr [0xdbd], 0
009DFB:  B9 09 00                     mov      cx, 9
009DFE:  BF BE 0D                     mov      di, 0xdbe
009E01:  F2 AE                        repne scasb al, byte ptr es:[di]
009E03:  E3 05                        jcxz     0x9e0a
009E05:  C6 06 BD 0D 01               mov      byte ptr [0xdbd], 1
009E0A:  83 F8 02                     cmp      ax, 2
009E0D:  74 05                        je       0x9e14
009E0F:  83 F8 07                     cmp      ax, 7
009E12:  75 07                        jne      0x9e1b
009E14:  C6 06 B9 0D 01               mov      byte ptr [0xdb9], 1
009E19:  EB 3C                        jmp      0x9e57
009E1B:  83 F8 00                     cmp      ax, 0
009E1E:  74 2D                        je       0x9e4d
009E20:  83 F8 01                     cmp      ax, 1
009E23:  74 28                        je       0x9e4d
009E25:  83 F8 2B                     cmp      ax, 0x2b
009E28:  74 23                        je       0x9e4d
009E2A:  83 F8 06                     cmp      ax, 6
009E2D:  74 1E                        je       0x9e4d
009E2F:  83 F8 05                     cmp      ax, 5
009E32:  74 19                        je       0x9e4d
009E34:  83 F8 29                     cmp      ax, 0x29
009E37:  74 14                        je       0x9e4d
009E39:  83 F8 2A                     cmp      ax, 0x2a
009E3C:  74 0F                        je       0x9e4d
009E3E:  83 F8 03                     cmp      ax, 3
009E41:  74 0A                        je       0x9e4d
009E43:  83 F8 2C                     cmp      ax, 0x2c
009E46:  74 05                        je       0x9e4d
009E48:  83 F8 04                     cmp      ax, 4
009E4B:  75 0A                        jne      0x9e57
009E4D:  80 0E AA 67 02               or       byte ptr [0x67aa], 2
009E52:  C6 06 BB 0D 01               mov      byte ptr [0xdbb], 1
009E57:  C6 06 BC 0D 00               mov      byte ptr [0xdbc], 0
009E5C:  83 F8 08                     cmp      ax, 8
009E5F:  75 12                        jne      0x9e73
009E61:  8B 1E 56 0A                  mov      bx, word ptr [0xa56]
009E65:  03 1E 58 0A                  add      bx, word ptr [0xa58]
009E69:  83 FB FE                     cmp      bx, -2
009E6C:  74 05                        je       0x9e73
009E6E:  C6 06 BC 0D 01               mov      byte ptr [0xdbc], 1
009E73:  E8 E9 02                     call     0xa15f
009E76:  A0 2A 25                     mov      al, byte ptr [0x252a]
009E79:  0A 06 4F 27                  or       al, byte ptr [0x274f]
009E7D:  0F 84 C9 00                  je       0x9f4a
009E81:  A1 88 67                     mov      ax, word ptr [0x6788]
009E84:  3B 06 8A 67                  cmp      ax, word ptr [0x678a]
009E88:  0F 84 BE 00                  je       0x9f4a
009E8C:  A3 8A 67                     mov      word ptr [0x678a], ax
009E8F:  BF 11 5F                     mov      di, 0x5f11
009E92:  B8 CE FF                     mov      ax, 0xffce
009E95:  33 DB                        xor      bx, bx
009E97:  8B CB                        mov      cx, bx
009E99:  8B D3                        mov      dx, bx
009E9B:  9A 00 00 CE 01               lcall    0x1ce, 0
009EA0:  E9 A7 00                     jmp      0x9f4a
009EA3:  F6 06 2D 25 01               test     byte ptr [0x252d], 1
009EA8:  0F 85 9E 00                  jne      0x9f4a
009EAC:  E8 05 03                     call     0xa1b4
009EAF:  E8 59 05                     call     0xa40b
009EB2:  74 5E                        je       0x9f12
009EB4:  F6 06 F3 24 08               test     byte ptr [0x24f3], 8
009EB9:  74 05                        je       0x9ec0
009EBB:  C6 06 D8 27 01               mov      byte ptr [0x27d8], 1
009EC0:  83 3E 88 67 05               cmp      word ptr [0x6788], 5
009EC5:  75 1F                        jne      0x9ee6
009EC7:  33 C0                        xor      ax, ax
009EC9:  C7 06 39 52 23 00            mov      word ptr [0x5239], 0x23
009ECF:  C7 06 3B 52 A5 00            mov      word ptr [0x523b], 0xa5
009ED5:  9A EB 0D 99 02               lcall    0x299, 0xdeb
009EDA:  C7 06 39 52 00 00            mov      word ptr [0x5239], 0
009EE0:  C7 06 3B 52 C8 00            mov      word ptr [0x523b], 0xc8
009EE6:  F6 06 E3 0A 01               test     byte ptr [0xae3], 1
009EEB:  0F 95 06 E4 0A               setne    byte ptr [0xae4]
009EF0:  F6 06 BD 67 01               test     byte ptr [0x67bd], 1
009EF5:  0F 95 06 13 0B               setne    byte ptr [0xb13]
009EFA:  C6 06 B2 1F 00               mov      byte ptr [0x1fb2], 0
009EFF:  A1 88 67                     mov      ax, word ptr [0x6788]
009F02:  A3 8A 67                     mov      word ptr [0x678a], ax
009F05:  C7 06 88 67 FF FF            mov      word ptr [0x6788], 0xffff
009F0B:  80 26 AA 67 FD               and      byte ptr [0x67aa], 0xfd
009F10:  EB 38                        jmp      0x9f4a
009F12:  83 3E 88 67 27               cmp      word ptr [0x6788], 0x27
009F17:  75 14                        jne      0x9f2d
009F19:  A1 AF 0D                     mov      ax, word ptr [0xdaf]
009F1C:  2B 06 60 0D                  sub      ax, word ptr [0xd60]
009F20:  83 F8 14                     cmp      ax, 0x14
009F23:  75 25                        jne      0x9f4a
009F25:  C7 06 4F 52 00 00            mov      word ptr [0x524f], 0
009F2B:  EB 1D                        jmp      0x9f4a
009F2D:  F6 06 F3 24 08               test     byte ptr [0x24f3], 8
009F32:  74 16                        je       0x9f4a
009F34:  A1 AF 0D                     mov      ax, word ptr [0xdaf]
009F37:  2B 06 60 0D                  sub      ax, word ptr [0xd60]
009F3B:  83 F8 08                     cmp      ax, 8
009F3E:  75 0A                        jne      0x9f4a
009F40:  C6 06 2F 25 01               mov      byte ptr [0x252f], 1
009F45:  C6 06 31 25 06               mov      byte ptr [0x2531], 6
009F4A:  5D                           pop      bp
009F4B:  07                           pop      es
009F4C:  5E                           pop      si
009F4D:  5F                           pop      di
009F4E:  5A                           pop      dx
009F4F:  59                           pop      cx
009F50:  5B                           pop      bx
009F51:  58                           pop      ax
009F52:  CB                           retf    
