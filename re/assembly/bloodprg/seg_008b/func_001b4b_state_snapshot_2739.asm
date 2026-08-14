; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001b4b
; seg_off: 008b:0c9b
; group: seg_008b
; provenance: recursive_graph
; label: save_load_menu_step
; label_comment: Complete save/load menu and persistence coordinator. It animates and edits the ten-entry DS:0x25ED slot directory, reserves index nine for the DS:0x2739 quicksave path, writes profile/state/string/runtime-object/patch blocks in that order, restores the same blocks before rebuilding VM-derived state and HUD data, and clears both mode gates on terminal success or failure. Natural C and direct vectors: re/source/bloodprg/candidates/seg_008b/func_001b4b_save_load_menu_step.c and re/tools/oracle_vectors/func_1b4b_natural.json
; byte_count: 553
; boundary: cfg_blocks_26_terminals_5
; terminal: jmp 0x1bfc:1, jmp 0x1c3f:1, jmp 0x1d5b:1, jmp 0x1d6d:1, ret:1
; direct_callees: 0x001d74, 0x001d94, 0x001dd8, 0x001e5d
; indirect_calls: 11
; routine_bytes_sha256: aac51f24e2f04b7eb02464e61d41a75b61ce0130f9687b5cd5eb3429a7e81dab

001B4B:  56                           push     si
001B4C:  57                           push     di
001B4D:  55                           push     bp
001B4E:  06                           push     es
001B4F:  1E                           push     ds
001B50:  52                           push     dx
001B51:  F6 06 39 27 01               test     byte ptr [0x2739], 1
001B56:  74 18                        je       0x1b70
001B58:  BE 61 01                     mov      si, 0x161
001B5B:  BF 0D 27                     mov      di, 0x270d
001B5E:  89 3E 34 27                  mov      word ptr [0x2734], di
001B62:  B9 02 00                     mov      cx, 2
001B65:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
001B68:  C6 06 39 27 00               mov      byte ptr [0x2739], 0
001B6D:  E9 CF 00                     jmp      0x1c3f
001B70:  A0 37 27                     mov      al, byte ptr [0x2737]
001B73:  0A 06 36 27                  or       al, byte ptr [0x2736]
001B77:  0F 84 F2 01                  je       0x1d6d
001B7B:  80 0E 93 27 04               or       byte ptr [0x2793], 4
001B80:  BE D7 25                     mov      si, 0x25d7
001B83:  F6 06 38 27 01               test     byte ptr [0x2738], 1
001B88:  74 3B                        je       0x1bc5
001B8A:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
001B8F:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
001B94:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
001B99:  C6 06 DC 0A 00               mov      byte ptr [0xadc], 0
001B9E:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
001BA3:  C6 06 DA 0A 06               mov      byte ptr [0xada], 6
001BA8:  B8 ED 25                     mov      ax, 0x25ed
001BAB:  A3 34 27                     mov      word ptr [0x2734], ax
001BAE:  C7 06 32 27 00 00            mov      word ptr [0x2732], 0
001BB4:  56                           push     si
001BB5:  8B F0                        mov      si, ax
001BB7:  B9 04 00                     mov      cx, 4
001BBA:  BF 3B 27                     mov      di, 0x273b
001BBD:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
001BC0:  FE 06 38 27                  inc      byte ptr [0x2738]
001BC4:  5E                           pop      si
001BC5:  F6 06 38 27 02               test     byte ptr [0x2738], 2
001BCA:  74 15                        je       0x1be1
001BCC:  56                           push     si
001BCD:  BE AB 2A                     mov      si, 0x2aab
001BD0:  BF CF 25                     mov      di, 0x25cf
001BD3:  0E                           push     cs
001BD4:  E8 86 02                     call     0x1e5d
001BD7:  5E                           pop      si
001BD8:  0F 83 91 01                  jae      0x1d6d
001BDC:  C6 06 38 27 00               mov      byte ptr [0x2738], 0
001BE1:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
001BE6:  BD D7 25                     mov      bp, 0x25d7
001BE9:  F6 06 37 27 01               test     byte ptr [0x2737], 1
001BEE:  0F 85 CB 00                  jne      0x1cbd
001BF2:  50                           push     ax
001BF3:  C7 06 2E 27 00 00            mov      word ptr [0x272e], 0
001BF9:  BE 3B 27                     mov      si, 0x273b
001BFC:  AC                           lodsb    al, byte ptr [si]
001BFD:  0A C0                        or       al, al
001BFF:  74 0A                        je       0x1c0b
001C01:  3C 20                        cmp      al, 0x20
001C03:  74 06                        je       0x1c0b
001C05:  FF 06 2E 27                  inc      word ptr [0x272e]
001C09:  EB F1                        jmp      0x1bfc
001C0B:  58                           pop      ax
001C0C:  E8 C9 01                     call     0x1dd8
001C0F:  72 2E                        jb       0x1c3f
001C11:  0B C0                        or       ax, ax
001C13:  0F 88 56 01                  js       0x1d6d
001C17:  83 F8 09                     cmp      ax, 9
001C1A:  0F 84 4F 01                  je       0x1d6d
001C1E:  A3 32 27                     mov      word ptr [0x2732], ax
001C21:  03 C0                        add      ax, ax
001C23:  03 E8                        add      bp, ax
001C25:  8B 76 00                     mov      si, word ptr [bp]
001C28:  83 FE FF                     cmp      si, -1
001C2B:  0F 84 2C 01                  je       0x1d5b
001C2F:  89 36 34 27                  mov      word ptr [0x2734], si
001C33:  B9 04 00                     mov      cx, 4
001C36:  BF 3B 27                     mov      di, 0x273b
001C39:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
001C3C:  E9 2E 01                     jmp      0x1d6d
001C3F:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
001C44:  C4 3E 24 67                  les      di, ptr [0x6724]
001C48:  8B 36 34 27                  mov      si, word ptr [0x2734]
001C4C:  83 C6 10                     add      si, 0x10
001C4F:  8B D6                        mov      dx, si
001C51:  B8 00 3C                     mov      ax, 0x3c00
001C54:  33 C9                        xor      cx, cx
001C56:  CD 21                        int      0x21
001C58:  0F 82 FF 00                  jb       0x1d5b
001C5C:  8B D8                        mov      bx, ax
001C5E:  B4 40                        mov      ah, 0x40
001C60:  B9 02 00                     mov      cx, 2
001C63:  BA 7E 67                     mov      dx, 0x677e
001C66:  CD 21                        int      0x21
001C68:  B4 40                        mov      ah, 0x40
001C6A:  B9 00 02                     mov      cx, 0x200
001C6D:  BA DE 6A                     mov      dx, 0x6ade
001C70:  CD 21                        int      0x21
001C72:  BA DE 6C                     mov      dx, 0x6cde
001C75:  B9 60 00                     mov      cx, 0x60
001C78:  B4 40                        mov      ah, 0x40
001C7A:  CD 21                        int      0x21
001C7C:  A1 16 67                     mov      ax, word ptr [0x6716]
001C7F:  9A AC 01 B9 04               lcall    0x4b9, 0x1ac
001C84:  8B C8                        mov      cx, ax
001C86:  B4 40                        mov      ah, 0x40
001C88:  C5 16 24 67                  lds      dx, ptr [0x6724]
001C8C:  CD 21                        int      0x21
001C8E:  E8 03 01                     call     0x1d94
001C91:  8B C8                        mov      cx, ax
001C93:  65 C5 16 BC 0A               lds      dx, ptr gs:[0xabc]
001C98:  B4 40                        mov      ah, 0x40
001C9A:  CD 21                        int      0x21
001C9C:  B4 3E                        mov      ah, 0x3e
001C9E:  CD 21                        int      0x21
001CA0:  8C E8                        mov      ax, gs
001CA2:  8E D8                        mov      ds, ax
001CA4:  8E C0                        mov      es, ax
001CA6:  BE FC 00                     mov      si, 0xfc
001CA9:  BF ED 25                     mov      di, 0x25ed
001CAC:  66 B8 40 01 00 00            mov      eax, 0x140
001CB2:  9A 8B 08 CE 01               lcall    0x1ce, 0x88b
001CB7:  66 33 C0                     xor      eax, eax
001CBA:  E9 9E 00                     jmp      0x1d5b
001CBD:  0B C0                        or       ax, ax
001CBF:  0F 88 AA 00                  js       0x1d6d
001CC3:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
001CC8:  03 C0                        add      ax, ax
001CCA:  03 E8                        add      bp, ax
001CCC:  8B 76 00                     mov      si, word ptr [bp]
001CCF:  83 FE FF                     cmp      si, -1
001CD2:  0F 84 85 00                  je       0x1d5b
001CD6:  C4 3E 24 67                  les      di, ptr [0x6724]
001CDA:  83 C6 10                     add      si, 0x10
001CDD:  8B D6                        mov      dx, si
001CDF:  B8 00 3D                     mov      ax, 0x3d00
001CE2:  CD 21                        int      0x21
001CE4:  72 75                        jb       0x1d5b
001CE6:  8B D8                        mov      bx, ax
001CE8:  B9 02 00                     mov      cx, 2
001CEB:  BA 80 67                     mov      dx, 0x6780
001CEE:  B4 3F                        mov      ah, 0x3f
001CF0:  CD 21                        int      0x21
001CF2:  A1 80 67                     mov      ax, word ptr [0x6780]
001CF5:  9A 00 00 DA 04               lcall    0x4da, 0
001CFA:  C7 06 80 67 FF FF            mov      word ptr [0x6780], 0xffff
001D00:  C6 06 A8 67 01               mov      byte ptr [0x67a8], 1
001D05:  9A 04 02 DA 04               lcall    0x4da, 0x204
001D0A:  B9 00 02                     mov      cx, 0x200
001D0D:  B4 3F                        mov      ah, 0x3f
001D0F:  BA DE 6A                     mov      dx, 0x6ade
001D12:  CD 21                        int      0x21
001D14:  B9 60 00                     mov      cx, 0x60
001D17:  BA DE 6C                     mov      dx, 0x6cde
001D1A:  B4 3F                        mov      ah, 0x3f
001D1C:  CD 21                        int      0x21
001D1E:  A1 16 67                     mov      ax, word ptr [0x6716]
001D21:  9A AC 01 B9 04               lcall    0x4b9, 0x1ac
001D26:  8B C8                        mov      cx, ax
001D28:  B4 3F                        mov      ah, 0x3f
001D2A:  C5 16 24 67                  lds      dx, ptr [0x6724]
001D2E:  CD 21                        int      0x21
001D30:  65 C5 16 BC 0A               lds      dx, ptr gs:[0xabc]
001D35:  B4 3F                        mov      ah, 0x3f
001D37:  B9 FF FF                     mov      cx, 0xffff
001D3A:  CD 21                        int      0x21
001D3C:  E8 35 00                     call     0x1d74
001D3F:  8C E8                        mov      ax, gs
001D41:  8E D8                        mov      ds, ax
001D43:  9A BB 01 DA 04               lcall    0x4da, 0x1bb
001D48:  9A B6 14 1E 07               lcall    0x71e, 0x14b6
001D4D:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
001D52:  C6 06 55 5B 01               mov      byte ptr [0x5b55], 1
001D57:  B4 3E                        mov      ah, 0x3e
001D59:  CD 21                        int      0x21
001D5B:  65 80 26 93 27 FB            and      byte ptr gs:[0x2793], 0xfb
001D61:  65 C6 06 36 27 00            mov      byte ptr gs:[0x2736], 0
001D67:  65 C6 06 37 27 00            mov      byte ptr gs:[0x2737], 0
001D6D:  5A                           pop      dx
001D6E:  1F                           pop      ds
001D6F:  07                           pop      es
001D70:  5D                           pop      bp
001D71:  5F                           pop      di
001D72:  5E                           pop      si
001D73:  C3                           ret     
