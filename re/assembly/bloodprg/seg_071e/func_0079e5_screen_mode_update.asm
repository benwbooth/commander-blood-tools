; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0079e5
; seg_off: 071e:0205
; group: seg_071e
; provenance: recursive_graph
; label: screen_mode_update
; label_comment: presentation box-open animation driver: gated [0x27e1]&1; phase counter [0x2b93]; phases 1..6 index box table gs:0x2b97, phases 7..9 use full-screen path (si=0x6011). Writes subtitle Y [0x5e5e]. Palette 0xE0 fill/0xEF frame || MERGED 2026-07-25 (audit-fixes #130), also recorded as: screen-mode update gate: test [0x27e1]&1; es=gs. Guards a screen/mode-specific per-frame update path
; byte_count: 719
; boundary: cfg_blocks_38_terminals_13
; terminal: jmp 0x7b6d:1, jmp 0x7b80:1, jmp 0x7c18:1, jmp 0x7cad:9, ret:1
; direct_callees: 0x007cb4, 0x007ce8, 0x008c96
; indirect_calls: 22
; cxx_source: re/borland/bloodprg/seg_071e/func_0079e5_screen_mode_update.cpp
; routine_bytes_sha256: 3fa1e4fab8d4165c36fc55a7b0541e4304c90a17324f9c78e6f9ecca2e8bbd54

0079E5:  50                           push     ax
0079E6:  53                           push     bx
0079E7:  51                           push     cx
0079E8:  52                           push     dx
0079E9:  55                           push     bp
0079EA:  56                           push     si
0079EB:  F6 06 E1 27 01               test     byte ptr [0x27e1], 1
0079F0:  0F 84 B9 02                  je       0x7cad
0079F4:  8C E8                        mov      ax, gs
0079F6:  8E C0                        mov      es, ax
0079F8:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
0079FD:  0F 85 6C 01                  jne      0x7b6d
007A01:  A1 93 2B                     mov      ax, word ptr [0x2b93]
007A04:  0B C0                        or       ax, ax
007A06:  75 2A                        jne      0x7a32
007A08:  C7 06 5E 5E 01 00            mov      word ptr [0x5e5e], 1
007A0E:  C6 06 E3 27 00               mov      byte ptr [0x27e3], 0
007A13:  C7 06 36 0A 0F 00            mov      word ptr [0xa36], 0xf
007A19:  C7 06 93 2B 01 00            mov      word ptr [0x2b93], 1
007A1F:  B8 1F 00                     mov      ax, 0x1f
007A22:  9A 41 12 99 02               lcall    0x299, 0x1241
007A27:  B8 01 00                     mov      ax, 1
007A2A:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
007A2F:  E9 7B 02                     jmp      0x7cad
007A32:  83 F8 64                     cmp      ax, 0x64
007A35:  0F 8D 07 02                  jge      0x7c40
007A39:  48                           dec      ax
007A3A:  83 F8 06                     cmp      ax, 6
007A3D:  7D 2B                        jge      0x7a6a
007A3F:  FF 06 93 2B                  inc      word ptr [0x2b93]
007A43:  BE 97 2B                     mov      si, 0x2b97
007A46:  C1 E0 03                     shl      ax, 3
007A49:  03 F0                        add      si, ax
007A4B:  AD                           lodsw    ax, word ptr [si]
007A4C:  8B D8                        mov      bx, ax
007A4E:  AD                           lodsw    ax, word ptr [si]
007A4F:  8B C8                        mov      cx, ax
007A51:  AD                           lodsw    ax, word ptr [si]
007A52:  8B D0                        mov      dx, ax
007A54:  AD                           lodsw    ax, word ptr [si]
007A55:  8B E8                        mov      bp, ax
007A57:  B8 E0 00                     mov      ax, 0xe0
007A5A:  9A DC 0C 99 02               lcall    0x299, 0xcdc
007A5F:  B8 EF 00                     mov      ax, 0xef
007A62:  9A B5 0B 99 02               lcall    0x299, 0xbb5
007A67:  E9 43 02                     jmp      0x7cad
007A6A:  83 E8 06                     sub      ax, 6
007A6D:  83 F8 03                     cmp      ax, 3
007A70:  7D 2A                        jge      0x7a9c
007A72:  FF 06 93 2B                  inc      word ptr [0x2b93]
007A76:  BB 00 00                     mov      bx, 0
007A79:  8B CB                        mov      cx, bx
007A7B:  BA 40 01                     mov      dx, 0x140
007A7E:  BD C8 00                     mov      bp, 0xc8
007A81:  BE 11 60                     mov      si, 0x6011
007A84:  9A 0E 04 99 02               lcall    0x299, 0x40e
007A89:  43                           inc      bx
007A8A:  B9 0A 00                     mov      cx, 0xa
007A8D:  4A                           dec      dx
007A8E:  BD 82 00                     mov      bp, 0x82
007A91:  B8 03 00                     mov      ax, 3
007A94:  9A F5 0B 99 02               lcall    0x299, 0xbf5
007A99:  E9 11 02                     jmp      0x7cad
007A9C:  BB 00 00                     mov      bx, 0
007A9F:  8B CB                        mov      cx, bx
007AA1:  BA 40 01                     mov      dx, 0x140
007AA4:  BD C8 00                     mov      bp, 0xc8
007AA7:  BE 11 60                     mov      si, 0x6011
007AAA:  9A 0E 04 99 02               lcall    0x299, 0x40e
007AAF:  66 FF 36 21 52               push     dword ptr [0x5221]
007AB4:  66 FF 36 29 52               push     dword ptr [0x5229]
007AB9:  66 8F 06 21 52               pop      dword ptr [0x5221]
007ABE:  66 8F 06 29 52               pop      dword ptr [0x5229]
007AC3:  BB 00 00                     mov      bx, 0
007AC6:  8B CB                        mov      cx, bx
007AC8:  BA 40 01                     mov      dx, 0x140
007ACB:  BD C8 00                     mov      bp, 0xc8
007ACE:  BE 11 60                     mov      si, 0x6011
007AD1:  9A 0E 04 99 02               lcall    0x299, 0x40e
007AD6:  B8 00 00                     mov      ax, 0
007AD9:  BD 8C 00                     mov      bp, 0x8c
007ADC:  8B C8                        mov      cx, ax
007ADE:  9A DC 0C 99 02               lcall    0x299, 0xcdc
007AE3:  66 FF 36 21 52               push     dword ptr [0x5221]
007AE8:  66 FF 36 29 52               push     dword ptr [0x5229]
007AED:  66 8F 06 21 52               pop      dword ptr [0x5221]
007AF2:  66 8F 06 29 52               pop      dword ptr [0x5229]
007AF7:  A0 E3 27                     mov      al, byte ptr [0x27e3]
007AFA:  98                           cwde    
007AFB:  C1 E0 04                     shl      ax, 4
007AFE:  BE DE 6C                     mov      si, 0x6cde
007B01:  03 F0                        add      si, ax
007B03:  8A 04                        mov      al, byte ptr [si]
007B05:  0A C0                        or       al, al
007B07:  75 23                        jne      0x7b2c
007B09:  BD 82 00                     mov      bp, 0x82
007B0C:  BB 01 00                     mov      bx, 1
007B0F:  BA 3F 01                     mov      dx, 0x13f
007B12:  B9 0A 00                     mov      cx, 0xa
007B15:  B8 03 00                     mov      ax, 3
007B18:  9A F5 0B 99 02               lcall    0x299, 0xbf5
007B1D:  E8 94 01                     call     0x7cb4
007B20:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
007B25:  0F 85 A0 00                  jne      0x7bc9
007B29:  E9 81 01                     jmp      0x7cad
007B2C:  8B FE                        mov      di, si
007B2E:  9A 69 20 DA 04               lcall    0x4da, 0x2069
007B33:  F6 06 A1 0B 01               test     byte ptr [0xba1], 1
007B38:  74 0D                        je       0x7b47
007B3A:  9A ED 03 1B 0B               lcall    0xb1b, 0x3ed
007B3F:  BE 2D 0D                     mov      si, 0xd2d
007B42:  9A 07 06 1B 0B               lcall    0xb1b, 0x607
007B47:  9A 03 04 1B 0B               lcall    0xb1b, 0x403
007B4C:  A0 DE 6C                     mov      al, byte ptr [0x6cde]
007B4F:  98                           cwde    
007B50:  C1 E0 07                     shl      ax, 7
007B53:  05 20 13                     add      ax, 0x1320
007B56:  A3 1A 13                     mov      word ptr [0x131a], ax
007B59:  A0 1E 13                     mov      al, byte ptr [0x131e]
007B5C:  A2 1F 13                     mov      byte ptr [0x131f], al
007B5F:  C7 06 A7 1F 0A 00            mov      word ptr [0x1fa7], 0xa
007B65:  C7 06 1C 13 00 00            mov      word ptr [0x131c], 0
007B6B:  EB 13                        jmp      0x7b80
007B6D:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
007B72:  75 55                        jne      0x7bc9
007B74:  9A 00 00 71 09               lcall    0x971, 0
007B79:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
007B7E:  75 39                        jne      0x7bb9
007B80:  80 3E 1F 13 00               cmp      byte ptr [0x131f], 0
007B85:  74 1E                        je       0x7ba5
007B87:  FE 0E 1F 13                  dec      byte ptr [0x131f]
007B8B:  BF 9E 20                     mov      di, 0x209e
007B8E:  8B 36 1A 13                  mov      si, word ptr [0x131a]
007B92:  83 06 1A 13 10               add      word ptr [0x131a], 0x10
007B97:  AC                           lodsb    al, byte ptr [si]
007B98:  AA                           stosb    byte ptr es:[di], al
007B99:  0A C0                        or       al, al
007B9B:  75 FA                        jne      0x7b97
007B9D:  C7 06 88 67 02 00            mov      word ptr [0x6788], 2
007BA3:  EB C8                        jmp      0x7b6d
007BA5:  B8 1A 0F                     mov      ax, 0xf1a
007BA8:  A3 18 0F                     mov      word ptr [0xf18], ax
007BAB:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
007BB0:  C7 06 93 2B 07 00            mov      word ptr [0x2b93], 7
007BB6:  E9 F4 00                     jmp      0x7cad
007BB9:  F6 06 B8 0D 01               test     byte ptr [0xdb8], 1
007BBE:  74 06                        je       0x7bc6
007BC0:  E8 25 01                     call     0x7ce8
007BC3:  E8 EE 00                     call     0x7cb4
007BC6:  E9 E4 00                     jmp      0x7cad
007BC9:  F6 06 E0 27 01               test     byte ptr [0x27e0], 1
007BCE:  74 14                        je       0x7be4
007BD0:  B8 6A 00                     mov      ax, 0x6a
007BD3:  A3 93 2B                     mov      word ptr [0x2b93], ax
007BD6:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
007BDB:  74 3B                        je       0x7c18
007BDD:  9A 43 02 71 09               lcall    0x971, 0x243
007BE2:  EB 34                        jmp      0x7c18
007BE4:  C7 06 34 0A 00 00            mov      word ptr [0xa34], 0
007BEA:  C7 06 32 0A 0E 00            mov      word ptr [0xa32], 0xe
007BF0:  B8 01 00                     mov      ax, 1
007BF3:  9A ED 03 1B 0B               lcall    0xb1b, 0x3ed
007BF8:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
007BFD:  9A 43 02 71 09               lcall    0x971, 0x243
007C02:  FE 06 E3 27                  inc      byte ptr [0x27e3]
007C06:  80 3E E3 27 06               cmp      byte ptr [0x27e3], 6
007C0B:  75 05                        jne      0x7c12
007C0D:  C6 06 E3 27 00               mov      byte ptr [0x27e3], 0
007C12:  C7 06 93 2B 07 00            mov      word ptr [0x2b93], 7
007C18:  66 FF 36 21 52               push     dword ptr [0x5221]
007C1D:  66 A1 29 52                  mov      eax, dword ptr [0x5229]
007C21:  66 A3 21 52                  mov      dword ptr [0x5221], eax
007C25:  66 33 C0                     xor      eax, eax
007C28:  BD 82 00                     mov      bp, 0x82
007C2B:  BB 00 00                     mov      bx, 0
007C2E:  BA 40 01                     mov      dx, 0x140
007C31:  B9 0A 00                     mov      cx, 0xa
007C34:  9A DC 0C 99 02               lcall    0x299, 0xcdc
007C39:  66 8F 06 21 52               pop      dword ptr [0x5221]
007C3E:  EB 6D                        jmp      0x7cad
007C40:  83 E8 64                     sub      ax, 0x64
007C43:  75 3F                        jne      0x7c84
007C45:  C7 06 A7 1F 00 00            mov      word ptr [0x1fa7], 0
007C4B:  C6 06 E1 27 00               mov      byte ptr [0x27e1], 0
007C50:  80 26 93 27 FB               and      byte ptr [0x2793], 0xfb
007C55:  C7 06 93 2B 00 00            mov      word ptr [0x2b93], 0
007C5B:  C6 06 EB 27 01               mov      byte ptr [0x27eb], 1
007C60:  C7 06 5E 5E 08 00            mov      word ptr [0x5e5e], 8
007C66:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
007C6B:  F6 06 E0 27 01               test     byte ptr [0x27e0], 1
007C70:  74 0C                        je       0x7c7e
007C72:  C6 06 B1 1F 0C               mov      byte ptr [0x1fb1], 0xc
007C77:  C6 06 E0 27 00               mov      byte ptr [0x27e0], 0
007C7C:  EB 2F                        jmp      0x7cad
007C7E:  0E                           push     cs
007C7F:  E8 14 10                     call     0x8c96
007C82:  EB 29                        jmp      0x7cad
007C84:  FF 0E 93 2B                  dec      word ptr [0x2b93]
007C88:  BE 97 2B                     mov      si, 0x2b97
007C8B:  48                           dec      ax
007C8C:  C1 E0 03                     shl      ax, 3
007C8F:  03 F0                        add      si, ax
007C91:  AD                           lodsw    ax, word ptr [si]
007C92:  8B D8                        mov      bx, ax
007C94:  AD                           lodsw    ax, word ptr [si]
007C95:  8B C8                        mov      cx, ax
007C97:  AD                           lodsw    ax, word ptr [si]
007C98:  8B D0                        mov      dx, ax
007C9A:  AD                           lodsw    ax, word ptr [si]
007C9B:  8B E8                        mov      bp, ax
007C9D:  B8 E0 00                     mov      ax, 0xe0
007CA0:  9A DC 0C 99 02               lcall    0x299, 0xcdc
007CA5:  B8 EF 00                     mov      ax, 0xef
007CA8:  9A B5 0B 99 02               lcall    0x299, 0xbb5
007CAD:  5E                           pop      si
007CAE:  5D                           pop      bp
007CAF:  5A                           pop      dx
007CB0:  59                           pop      cx
007CB1:  5B                           pop      bx
007CB2:  58                           pop      ax
007CB3:  C3                           ret     
