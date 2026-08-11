; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008cce
; seg_off: 071e:14ee
; group: seg_071e
; provenance: recursive_graph
; label: nav_camera_state_check
; label_comment: per-frame NAV/CAMERA update (gated [0x278B] view-state, [0x278A]&1). Calls object-count gate 0x4DA:0x1E7A; if nonzero runs object_population_loop 0x8D2A -> entity_object_populate 0x40D0, activating the loaded world objects incl. nav destinations 0x15..0x1F. So destinations appear only in the nav view with a world loaded; empty at the console (count 0). Head of the #3/#6 activation chain
; byte_count: 949
; boundary: cfg_blocks_71_terminals_12
; terminal: jmp 0x8d6e:1, jmp 0x8e59:1, jmp 0x8e75:2, jmp 0x8f2a:1, jmp 0x8f3c:1, jmp 0x8f44:1, jmp 0x8f62:1, jmp 0x9078:3, ret:1
; direct_callees: 0x008c96, 0x009083, 0x0092a3, 0x00933a, 0x009364, 0x00954a, 0x00981b
; indirect_calls: 17
; routine_bytes_sha256: 784167ae2bd32bc90fa2b0d00863a8c58c3058bd6cca8254c2a33dda68351900

008CCE:  66 50                        push     eax
008CD0:  53                           push     bx
008CD1:  51                           push     cx
008CD2:  52                           push     dx
008CD3:  55                           push     bp
008CD4:  1E                           push     ds
008CD5:  56                           push     si
008CD6:  06                           push     es
008CD7:  57                           push     di
008CD8:  8C E8                        mov      ax, gs
008CDA:  8E D8                        mov      ds, ax
008CDC:  80 3E 8B 27 00               cmp      byte ptr [0x278b], 0
008CE1:  75 0C                        jne      0x8cef
008CE3:  F6 06 8A 27 01               test     byte ptr [0x278a], 1
008CE8:  0F 84 8C 03                  je       0x9078
008CEC:  E9 73 02                     jmp      0x8f62
008CEF:  C7 06 BF 27 00 00            mov      word ptr [0x27bf], 0
008CF5:  F6 06 8A 27 01               test     byte ptr [0x278a], 1
008CFA:  0F 85 AA 01                  jne      0x8ea8
008CFE:  80 3E 8B 27 08               cmp      byte ptr [0x278b], 8
008D03:  0F 85 16 01                  jne      0x8e1d
008D07:  1E                           push     ds
008D08:  C4 3E BC 0A                  les      di, ptr [0xabc]
008D0C:  B8 00 A0                     mov      ax, 0xa000
008D0F:  8E D8                        mov      ds, ax
008D11:  BE 00 C0                     mov      si, 0xc000
008D14:  9A E0 0E 99 02               lcall    0x299, 0xee0
008D19:  1F                           pop      ds
008D1A:  66 8E 06 26 67               mov      es, word ptr [0x6726]
008D1F:  9A 7A 1E DA 04               lcall    0x4da, 0x1e7a
008D24:  0B C0                        or       ax, ax
008D26:  0F 84 8C 00                  je       0x8db6
008D2A:  A3 C1 27                     mov      word ptr [0x27c1], ax
008D2D:  8B C8                        mov      cx, ax
008D2F:  66 FF 36 21 52               push     dword ptr [0x5221]
008D34:  66 A1 BC 0A                  mov      eax, dword ptr [0xabc]
008D38:  66 A3 21 52                  mov      dword ptr [0x5221], eax
008D3C:  BD D3 2A                     mov      bp, 0x2ad3
008D3F:  33 C0                        xor      ax, ax
008D41:  BA 2C 00                     mov      dx, 0x2c
008D44:  C6 06 90 27 00               mov      byte ptr [0x2790], 0
008D49:  51                           push     cx
008D4A:  8B 7E 00                     mov      di, word ptr [bp]
008D4D:  26 83 7D 14 00               cmp      word ptr es:[di + 0x14], 0
008D52:  0F 94 06 8F 27               sete     byte ptr [0x278f]
008D57:  55                           push     bp
008D58:  33 ED                        xor      bp, bp
008D5A:  26 F7 05 00 01               test     word ptr es:[di], 0x100
008D5F:  74 03                        je       0x8d64
008D61:  45                           inc      bp
008D62:  EB 0A                        jmp      0x8d6e
008D64:  26 F7 05 10 00               test     word ptr es:[di], 0x10
008D69:  74 03                        je       0x8d6e
008D6B:  83 C5 02                     add      bp, 2
008D6E:  26 8B 5D 18                  mov      bx, word ptr es:[di + 0x18]
008D72:  26 8B 4D 1A                  mov      cx, word ptr es:[di + 0x1a]
008D76:  9A 40 11 99 02               lcall    0x299, 0x1140
008D7B:  F6 06 8F 27 01               test     byte ptr [0x278f], 1
008D80:  74 1B                        je       0x8d9d
008D82:  83 EB 03                     sub      bx, 3
008D85:  83 E9 03                     sub      cx, 3
008D88:  83 C5 03                     add      bp, 3
008D8B:  B8 05 00                     mov      ax, 5
008D8E:  02 06 90 27                  add      al, byte ptr [0x2790]
008D92:  FE 06 90 27                  inc      byte ptr [0x2790]
008D96:  9A 40 11 99 02               lcall    0x299, 0x1140
008D9B:  33 C0                        xor      ax, ax
008D9D:  BB 00 00                     mov      bx, 0
008DA0:  9A E1 14 99 02               lcall    0x299, 0x14e1
008DA5:  5D                           pop      bp
008DA6:  83 C5 02                     add      bp, 2
008DA9:  59                           pop      cx
008DAA:  E2 9D                        loop     0x8d49
008DAC:  9A 41 12 99 02               lcall    0x299, 0x1241
008DB1:  66 8F 06 21 52               pop      dword ptr [0x5221]
008DB6:  BA 2C 00                     mov      dx, 0x2c
008DB9:  65 8B 3E 52 67               mov      di, word ptr gs:[0x6752]
008DBE:  26 8B 5D 18                  mov      bx, word ptr es:[di + 0x18]
008DC2:  83 EB 10                     sub      bx, 0x10
008DC5:  79 02                        jns      0x8dc9
008DC7:  33 DB                        xor      bx, bx
008DC9:  26 8B 4D 1A                  mov      cx, word ptr es:[di + 0x1a]
008DCD:  83 E9 0D                     sub      cx, 0xd
008DD0:  79 02                        jns      0x8dd4
008DD2:  33 C9                        xor      cx, cx
008DD4:  26 8B 7D 16                  mov      di, word ptr es:[di + 0x16]
008DD8:  26 F7 05 00 01               test     word ptr es:[di], 0x100
008DDD:  74 06                        je       0x8de5
008DDF:  83 C3 05                     add      bx, 5
008DE2:  83 C1 02                     add      cx, 2
008DE5:  26 F7 05 10 00               test     word ptr es:[di], 0x10
008DEA:  74 03                        je       0x8def
008DEC:  83 C3 03                     add      bx, 3
008DEF:  BD 06 00                     mov      bp, 6
008DF2:  B8 01 00                     mov      ax, 1
008DF5:  9A 40 11 99 02               lcall    0x299, 0x1140
008DFA:  B8 01 00                     mov      ax, 1
008DFD:  9A 41 12 99 02               lcall    0x299, 0x1241
008E02:  33 C9                        xor      cx, cx
008E04:  8A 0E 90 27                  mov      cl, byte ptr [0x2790]
008E08:  E3 0B                        jcxz     0x8e15
008E0A:  B8 05 00                     mov      ax, 5
008E0D:  9A 41 12 99 02               lcall    0x299, 0x1241
008E12:  40                           inc      ax
008E13:  E2 F8                        loop     0x8e0d
008E15:  B8 1F 00                     mov      ax, 0x1f
008E18:  9A 41 12 99 02               lcall    0x299, 0x1241
008E1D:  BE 52 27                     mov      si, 0x2752
008E20:  A0 8B 27                     mov      al, byte ptr [0x278b]
008E23:  FE C8                        dec      al
008E25:  98                           cwde    
008E26:  0F 94 06 91 27               sete     byte ptr [0x2791]
008E2B:  C1 E0 02                     shl      ax, 2
008E2E:  03 F0                        add      si, ax
008E30:  E8 31 05                     call     0x9364
008E33:  8B 4C 02                     mov      cx, word ptr [si + 2]
008E36:  33 DB                        xor      bx, bx
008E38:  BA 40 01                     mov      dx, 0x140
008E3B:  1E                           push     ds
008E3C:  C5 36 21 52                  lds      si, ptr [0x5221]
008E40:  83 F9 6E                     cmp      cx, 0x6e
008E43:  7C 05                        jl       0x8e4a
008E45:  B9 6E 00                     mov      cx, 0x6e
008E48:  EB 2B                        jmp      0x8e75
008E4A:  51                           push     cx
008E4B:  B9 6E 00                     mov      cx, 0x6e
008E4E:  E8 E9 04                     call     0x933a
008E51:  41                           inc      cx
008E52:  81 F9 C8 00                  cmp      cx, 0xc8
008E56:  75 F6                        jne      0x8e4e
008E58:  59                           pop      cx
008E59:  AD                           lodsw    ax, word ptr [si]
008E5A:  0B C0                        or       ax, ax
008E5C:  78 36                        js       0x8e94
008E5E:  8B D0                        mov      dx, ax
008E60:  33 DB                        xor      bx, bx
008E62:  E8 D5 04                     call     0x933a
008E65:  AD                           lodsw    ax, word ptr [si]
008E66:  03 D0                        add      dx, ax
008E68:  8B DA                        mov      bx, dx
008E6A:  BA 40 01                     mov      dx, 0x140
008E6D:  2B D3                        sub      dx, bx
008E6F:  E8 C8 04                     call     0x933a
008E72:  41                           inc      cx
008E73:  EB E4                        jmp      0x8e59
008E75:  AD                           lodsw    ax, word ptr [si]
008E76:  8B D8                        mov      bx, ax
008E78:  AD                           lodsw    ax, word ptr [si]
008E79:  0B C0                        or       ax, ax
008E7B:  78 08                        js       0x8e85
008E7D:  8B D0                        mov      dx, ax
008E7F:  E8 B8 04                     call     0x933a
008E82:  41                           inc      cx
008E83:  EB F0                        jmp      0x8e75
008E85:  BA 40 01                     mov      dx, 0x140
008E88:  33 DB                        xor      bx, bx
008E8A:  E8 AD 04                     call     0x933a
008E8D:  41                           inc      cx
008E8E:  81 F9 C8 00                  cmp      cx, 0xc8
008E92:  72 F1                        jb       0x8e85
008E94:  1F                           pop      ds
008E95:  FE 0E 8B 27                  dec      byte ptr [0x278b]
008E99:  8C E8                        mov      ax, gs
008E9B:  8E C0                        mov      es, ax
008E9D:  BF 12 66                     mov      di, 0x6612
008EA0:  9A 0D 21 99 02               lcall    0x299, 0x210d
008EA5:  E9 D0 01                     jmp      0x9078
008EA8:  80 3E 8B 27 08               cmp      byte ptr [0x278b], 8
008EAD:  75 45                        jne      0x8ef4
008EAF:  C6 06 91 27 00               mov      byte ptr [0x2791], 0
008EB4:  B8 01 00                     mov      ax, 1
008EB7:  9A 41 12 99 02               lcall    0x299, 0x1241
008EBC:  33 C9                        xor      cx, cx
008EBE:  8A 0E 90 27                  mov      cl, byte ptr [0x2790]
008EC2:  E3 0B                        jcxz     0x8ecf
008EC4:  B8 05 00                     mov      ax, 5
008EC7:  9A 41 12 99 02               lcall    0x299, 0x1241
008ECC:  40                           inc      ax
008ECD:  E2 F8                        loop     0x8ec7
008ECF:  66 FF 36 29 52               push     dword ptr [0x5229]
008ED4:  66 A1 BC 0A                  mov      eax, dword ptr [0xabc]
008ED8:  66 A3 29 52                  mov      dword ptr [0x5229], eax
008EDC:  66 33 C0                     xor      eax, eax
008EDF:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
008EE4:  E8 34 09                     call     0x981b
008EE7:  0E                           push     cs
008EE8:  E8 AB FD                     call     0x8c96
008EEB:  0E                           push     cs
008EEC:  E8 5B 06                     call     0x954a
008EEF:  66 8F 06 29 52               pop      dword ptr [0x5229]
008EF4:  BE 52 27                     mov      si, 0x2752
008EF7:  A0 8B 27                     mov      al, byte ptr [0x278b]
008EFA:  2C 08                        sub      al, 8
008EFC:  F6 D8                        neg      al
008EFE:  FE C0                        inc      al
008F00:  98                           cwde    
008F01:  C1 E0 02                     shl      ax, 2
008F04:  03 F0                        add      si, ax
008F06:  E8 5B 04                     call     0x9364
008F09:  8B 4C 02                     mov      cx, word ptr [si + 2]
008F0C:  33 DB                        xor      bx, bx
008F0E:  BA 40 01                     mov      dx, 0x140
008F11:  1E                           push     ds
008F12:  C5 36 21 52                  lds      si, ptr [0x5221]
008F16:  83 F9 6E                     cmp      cx, 0x6e
008F19:  7C 05                        jl       0x8f20
008F1B:  B9 6E 00                     mov      cx, 0x6e
008F1E:  EB 1C                        jmp      0x8f3c
008F20:  51                           push     cx
008F21:  49                           dec      cx
008F22:  78 05                        js       0x8f29
008F24:  E8 13 04                     call     0x933a
008F27:  E2 FB                        loop     0x8f24
008F29:  59                           pop      cx
008F2A:  AD                           lodsw    ax, word ptr [si]
008F2B:  8B D8                        mov      bx, ax
008F2D:  AD                           lodsw    ax, word ptr [si]
008F2E:  0B C0                        or       ax, ax
008F30:  0F 88 60 FF                  js       0x8e94
008F34:  8B D0                        mov      dx, ax
008F36:  E8 01 04                     call     0x933a
008F39:  41                           inc      cx
008F3A:  EB EE                        jmp      0x8f2a
008F3C:  51                           push     cx
008F3D:  49                           dec      cx
008F3E:  E8 F9 03                     call     0x933a
008F41:  E2 FB                        loop     0x8f3e
008F43:  59                           pop      cx
008F44:  AD                           lodsw    ax, word ptr [si]
008F45:  0B C0                        or       ax, ax
008F47:  0F 88 49 FF                  js       0x8e94
008F4B:  8B D0                        mov      dx, ax
008F4D:  33 DB                        xor      bx, bx
008F4F:  E8 E8 03                     call     0x933a
008F52:  AD                           lodsw    ax, word ptr [si]
008F53:  03 D0                        add      dx, ax
008F55:  8B DA                        mov      bx, dx
008F57:  BA 40 01                     mov      dx, 0x140
008F5A:  2B D3                        sub      dx, bx
008F5C:  E8 DB 03                     call     0x933a
008F5F:  41                           inc      cx
008F60:  EB E2                        jmp      0x8f44
008F62:  F6 06 91 27 01               test     byte ptr [0x2791], 1
008F67:  0F 84 0D 01                  je       0x9078
008F6B:  80 0E 93 27 04               or       byte ptr [0x2793], 4
008F70:  83 3E BF 27 00               cmp      word ptr [0x27bf], 0
008F75:  0F 85 FC 00                  jne      0x9075
008F79:  33 C9                        xor      cx, cx
008F7B:  8A 0E 90 27                  mov      cl, byte ptr [0x2790]
008F7F:  E3 19                        jcxz     0x8f9a
008F81:  BD B2 62                     mov      bp, 0x62b2
008F84:  8A 46 00                     mov      al, byte ptr [bp]
008F87:  0C 03                        or       al, 3
008F89:  F6 06 3F 0B 01               test     byte ptr [0xb3f], 1
008F8E:  75 02                        jne      0x8f92
008F90:  24 FE                        and      al, 0xfe
008F92:  88 46 00                     mov      byte ptr [bp], al
008F95:  83 C5 20                     add      bp, 0x20
008F98:  E2 EA                        loop     0x8f84
008F9A:  A0 32 62                     mov      al, byte ptr [0x6232]
008F9D:  0C 03                        or       al, 3
008F9F:  F6 06 3F 0B 07               test     byte ptr [0xb3f], 7
008FA4:  74 02                        je       0x8fa8
008FA6:  24 FE                        and      al, 0xfe
008FA8:  A2 32 62                     mov      byte ptr [0x6232], al
008FAB:  66 8E 06 26 67               mov      es, word ptr [0x6726]
008FB0:  E8 F0 02                     call     0x92a3
008FB3:  0B C0                        or       ax, ax
008FB5:  0F 84 BF 00                  je       0x9078
008FB9:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
008FBE:  75 34                        jne      0x8ff4
008FC0:  8B F0                        mov      si, ax
008FC2:  1E                           push     ds
008FC3:  8C C0                        mov      ax, es
008FC5:  8E D8                        mov      ds, ax
008FC7:  83 C6 04                     add      si, 4
008FCA:  B8 01 00                     mov      ax, 1
008FCD:  9A 3D 01 99 02               lcall    0x299, 0x13d
008FD2:  65 8B 1E 2A 0A               mov      bx, word ptr gs:[0xa2a]
008FD7:  2B D8                        sub      bx, ax
008FD9:  79 02                        jns      0x8fdd
008FDB:  33 DB                        xor      bx, bx
008FDD:  65 8B 16 2C 0A               mov      dx, word ptr gs:[0xa2c]
008FE2:  83 EA 0A                     sub      dx, 0xa
008FE5:  79 02                        jns      0x8fe9
008FE7:  33 D2                        xor      dx, dx
008FE9:  B0 EF                        mov      al, 0xef
008FEB:  9A 02 02 99 02               lcall    0x299, 0x202
008FF0:  1F                           pop      ds
008FF1:  E9 84 00                     jmp      0x9078
008FF4:  C7 06 34 0A 00 00            mov      word ptr [0xa34], 0
008FFA:  C7 06 32 0A 0B 00            mov      word ptr [0xa32], 0xb
009000:  81 3E 2A 0A A0 00            cmp      word ptr [0xa2a], 0xa0
009006:  76 04                        jbe      0x900c
009008:  FF 06 32 0A                  inc      word ptr [0xa32]
00900C:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
009011:  C6 06 40 0A 00               mov      byte ptr [0xa40], 0
009016:  8B 1E 52 67                  mov      bx, word ptr [0x6752]
00901A:  83 C3 16                     add      bx, 0x16
00901D:  26 3B 07                     cmp      ax, word ptr es:[bx]
009020:  74 56                        je       0x9078
009022:  A3 BF 27                     mov      word ptr [0x27bf], ax
009025:  8C E8                        mov      ax, gs
009027:  8E C0                        mov      es, ax
009029:  BF AB 2A                     mov      di, 0x2aab
00902C:  A1 2A 0A                     mov      ax, word ptr [0xa2a]
00902F:  AB                           stosw    word ptr es:[di], ax
009030:  A1 2C 0A                     mov      ax, word ptr [0xa2c]
009033:  AB                           stosw    word ptr es:[di], ax
009034:  B8 04 00                     mov      ax, 4
009037:  AB                           stosw    word ptr es:[di], ax
009038:  AB                           stosw    word ptr es:[di], ax
009039:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
00903E:  C6 06 DA 0A 08               mov      byte ptr [0xada], 8
009043:  C6 06 88 27 01               mov      byte ptr [0x2788], 1
009048:  C6 06 89 27 00               mov      byte ptr [0x2789], 0
00904D:  C6 06 8C 27 01               mov      byte ptr [0x278c], 1
009052:  A1 BF 27                     mov      ax, word ptr [0x27bf]
009055:  A3 6A 67                     mov      word ptr [0x676a], ax
009058:  B8 01 00                     mov      ax, 1
00905B:  9A 41 12 99 02               lcall    0x299, 0x1241
009060:  33 C9                        xor      cx, cx
009062:  8A 0E 90 27                  mov      cl, byte ptr [0x2790]
009066:  E3 10                        jcxz     0x9078
009068:  B8 05 00                     mov      ax, 5
00906B:  9A 41 12 99 02               lcall    0x299, 0x1241
009070:  40                           inc      ax
009071:  E2 F8                        loop     0x906b
009073:  EB 03                        jmp      0x9078
009075:  E8 0B 00                     call     0x9083
009078:  5F                           pop      di
009079:  07                           pop      es
00907A:  5E                           pop      si
00907B:  1F                           pop      ds
00907C:  5D                           pop      bp
00907D:  5A                           pop      dx
00907E:  59                           pop      cx
00907F:  5B                           pop      bx
009080:  66 58                        pop      eax
009082:  C3                           ret     
