; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bdb7
; seg_off: 0b1b:0607
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: mixer_gated_proc_b
; label_comment: mixer-gated audio routine (sibling of 0xbc50): same gs:0xade + gs:0xba3 gate, bails to 0xbffa when either is clear. Second mixer-service entry
; incoming: call@0x00125f->0b1b:0607
; incoming: call@0x001f2a->0b1b:0607
; incoming: call@0x005c18->0b1b:0607
; incoming: call@0x007b42->0b1b:0607
; incoming: call@0x008917->0b1b:0607
; incoming: call@0x00b25c->0b1b:0607
; byte_count: 590
; boundary: cfg_blocks_35_terminals_3
; terminal: jmp 0xbf0a:1, jmp 0xbfd1:1, retf:1
; direct_callees: none
; indirect_calls: 5
; routine_bytes_sha256: ddea3b50a1ab2133d199f5c84fd96458bcf9c7a67446852276c53927875e4381

00BDB7:  50                           push     ax
00BDB8:  53                           push     bx
00BDB9:  51                           push     cx
00BDBA:  52                           push     dx
00BDBB:  06                           push     es
00BDBC:  1E                           push     ds
00BDBD:  56                           push     si
00BDBE:  57                           push     di
00BDBF:  66 55                        push     ebp
00BDC1:  65 F6 06 DE 0A 01            test     byte ptr gs:[0xade], 1
00BDC7:  0F 84 2F 02                  je       0xbffa
00BDCB:  65 F6 06 A3 0B 01            test     byte ptr gs:[0xba3], 1
00BDD1:  0F 84 25 02                  je       0xbffa
00BDD5:  8C E8                        mov      ax, gs
00BDD7:  8E C0                        mov      es, ax
00BDD9:  8B D6                        mov      dx, si
00BDDB:  9A B3 03 CE 01               lcall    0x1ce, 0x3b3
00BDE0:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
00BDE6:  75 1C                        jne      0xbe04
00BDE8:  9A EA 05 CE 01               lcall    0x1ce, 0x5ea
00BDED:  66 65 C7 06 8A 0A 00 00 00 00 mov      dword ptr gs:[0xa8a], 0
00BDF7:  66 65 89 2E 92 0A            mov      dword ptr gs:[0xa92], ebp
00BDFD:  B8 00 3D                     mov      ax, 0x3d00
00BE00:  CD 21                        int      0x21
00BE02:  8B D8                        mov      bx, ax
00BE04:  65 C7 06 A7 0B 00 00         mov      word ptr gs:[0xba7], 0
00BE0B:  65 C6 06 A1 0B 00            mov      byte ptr gs:[0xba1], 0
00BE11:  65 C6 06 A0 0B 01            mov      byte ptr gs:[0xba0], 1
00BE17:  1E                           push     ds
00BE18:  53                           push     bx
00BE19:  52                           push     dx
00BE1A:  8C E8                        mov      ax, gs
00BE1C:  8E D8                        mov      ds, ax
00BE1E:  BE 90 01                     mov      si, 0x190
00BE21:  BF 18 0E                     mov      di, 0xe18
00BE24:  8B C7                        mov      ax, di
00BE26:  83 C0 12                     add      ax, 0x12
00BE29:  A3 58 5E                     mov      word ptr [0x5e58], ax
00BE2C:  AC                           lodsb    al, byte ptr [si]
00BE2D:  AA                           stosb    byte ptr es:[di], al
00BE2E:  0A C0                        or       al, al
00BE30:  75 FA                        jne      0xbe2c
00BE32:  C6 06 E2 27 02               mov      byte ptr [0x27e2], 2
00BE37:  C7 06 65 5E 00 00            mov      word ptr [0x5e65], 0
00BE3D:  C6 06 BC 67 00               mov      byte ptr [0x67bc], 0
00BE42:  66 FF 36 19 52               push     dword ptr [0x5219]
00BE47:  66 A1 1D 52                  mov      eax, dword ptr [0x521d]
00BE4B:  66 A3 19 52                  mov      dword ptr [0x5219], eax
00BE4F:  66 33 C0                     xor      eax, eax
00BE52:  9A 15 1C 1E 07               lcall    0x71e, 0x1c15
00BE57:  66 8F 06 19 52               pop      dword ptr [0x5219]
00BE5C:  C6 06 E2 27 00               mov      byte ptr [0x27e2], 0
00BE61:  C6 06 B0 67 00               mov      byte ptr [0x67b0], 0
00BE66:  5A                           pop      dx
00BE67:  5B                           pop      bx
00BE68:  1F                           pop      ds
00BE69:  B8 00 42                     mov      ax, 0x4200
00BE6C:  66 33 C9                     xor      ecx, ecx
00BE6F:  65 8B 0E 8C 0A               mov      cx, word ptr gs:[0xa8c]
00BE74:  65 8B 16 8A 0A               mov      dx, word ptr gs:[0xa8a]
00BE79:  83 C2 1A                     add      dx, 0x1a
00BE7C:  CD 21                        int      0x21
00BE7E:  66 65 83 2E 92 0A 1A         sub      dword ptr gs:[0xa92], 0x1a
00BE85:  65 83 3E 60 0A FF            cmp      word ptr gs:[0xa60], -1
00BE8B:  74 5B                        je       0xbee8
00BE8D:  65 C6 06 9F 0B 00            mov      byte ptr gs:[0xb9f], 0
00BE93:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
00BE98:  65 C7 06 4E 0A 00 00         mov      word ptr gs:[0xa4e], 0
00BE9F:  53                           push     bx
00BEA0:  65 8B 1E 4E 0A               mov      bx, word ptr gs:[0xa4e]
00BEA5:  32 C0                        xor      al, al
00BEA7:  B9 02 00                     mov      cx, 2
00BEAA:  65 8B 16 60 0A               mov      dx, word ptr gs:[0xa60]
00BEAF:  B4 44                        mov      ah, 0x44
00BEB1:  CD 67                        int      0x67
00BEB3:  43                           inc      bx
00BEB4:  FE C0                        inc      al
00BEB6:  E2 F7                        loop     0xbeaf
00BEB8:  65 89 1E 4E 0A               mov      word ptr gs:[0xa4e], bx
00BEBD:  5B                           pop      bx
00BEBE:  33 D2                        xor      dx, dx
00BEC0:  B9 00 80                     mov      cx, 0x8000
00BEC3:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
00BEC8:  66 2B C1                     sub      eax, ecx
00BECB:  79 02                        jns      0xbecf
00BECD:  03 C8                        add      cx, ax
00BECF:  B4 3F                        mov      ah, 0x3f
00BED1:  CD 21                        int      0x21
00BED3:  65 83 06 A7 0B 02            add      word ptr gs:[0xba7], 2
00BED9:  66 0F B7 C0                  movzx    eax, ax
00BEDD:  66 65 29 06 92 0A            sub      dword ptr gs:[0xa92], eax
00BEE3:  75 BA                        jne      0xbe9f
00BEE5:  E9 E9 00                     jmp      0xbfd1
00BEE8:  65 83 3E 5E 0A FF            cmp      word ptr gs:[0xa5e], -1
00BEEE:  0F 84 81 00                  je       0xbf73
00BEF2:  65 C6 06 9F 0B 01            mov      byte ptr gs:[0xb9f], 1
00BEF8:  65 C5 16 B7 0B               lds      dx, ptr gs:[0xbb7]
00BEFD:  BE 6C 0A                     mov      si, 0xa6c
00BF00:  66 65 C7 06 4E 0A 00 00 00 00 mov      dword ptr gs:[0xa4e], 0
00BF0A:  B9 00 80                     mov      cx, 0x8000
00BF0D:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
00BF12:  66 2B C1                     sub      eax, ecx
00BF15:  79 02                        jns      0xbf19
00BF17:  03 C8                        add      cx, ax
00BF19:  B4 3F                        mov      ah, 0x3f
00BF1B:  CD 21                        int      0x21
00BF1D:  0B C0                        or       ax, ax
00BF1F:  0F 84 AE 00                  je       0xbfd1
00BF23:  50                           push     ax
00BF24:  66 0F B7 C0                  movzx    eax, ax
00BF28:  1E                           push     ds
00BF29:  53                           push     bx
00BF2A:  A8 01                        test     al, 1
00BF2C:  74 02                        je       0xbf30
00BF2E:  66 40                        inc      eax
00BF30:  8B FE                        mov      di, si
00BF32:  66 AB                        stosd    dword ptr es:[di], eax
00BF34:  33 C0                        xor      ax, ax
00BF36:  AB                           stosw    word ptr es:[di], ax
00BF37:  8B C2                        mov      ax, dx
00BF39:  AB                           stosw    word ptr es:[di], ax
00BF3A:  8C D8                        mov      ax, ds
00BF3C:  AB                           stosw    word ptr es:[di], ax
00BF3D:  06                           push     es
00BF3E:  1F                           pop      ds
00BF3F:  A1 5E 0A                     mov      ax, word ptr [0xa5e]
00BF42:  AB                           stosw    word ptr es:[di], ax
00BF43:  66 A1 4E 0A                  mov      eax, dword ptr [0xa4e]
00BF47:  66 AB                        stosd    dword ptr es:[di], eax
00BF49:  66 81 06 4E 0A 00 80 00 00   add      dword ptr [0xa4e], 0x8000
00BF52:  66 B8 00 0B 00 00            mov      eax, 0xb00
00BF58:  FF 1E 4A 0A                  lcall    [0xa4a]
00BF5C:  5B                           pop      bx
00BF5D:  1F                           pop      ds
00BF5E:  58                           pop      ax
00BF5F:  65 83 06 A7 0B 02            add      word ptr gs:[0xba7], 2
00BF65:  66 0F B7 C0                  movzx    eax, ax
00BF69:  66 65 29 06 92 0A            sub      dword ptr gs:[0xa92], eax
00BF6F:  74 60                        je       0xbfd1
00BF71:  EB 97                        jmp      0xbf0a
00BF73:  65 C6 06 9F 0B 02            mov      byte ptr gs:[0xb9f], 2
00BF79:  65 A1 49 0C                  mov      ax, word ptr gs:[0xc49]
00BF7D:  0B C0                        or       ax, ax
00BF7F:  74 08                        je       0xbf89
00BF81:  53                           push     bx
00BF82:  8B D8                        mov      bx, ax
00BF84:  B4 3E                        mov      ah, 0x3e
00BF86:  CD 21                        int      0x21
00BF88:  5B                           pop      bx
00BF89:  06                           push     es
00BF8A:  1F                           pop      ds
00BF8B:  33 C9                        xor      cx, cx
00BF8D:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
00BF92:  BA AE 00                     mov      dx, 0xae
00BF95:  B8 00 3C                     mov      ax, 0x3c00
00BF98:  CD 21                        int      0x21
00BF9A:  A3 49 0C                     mov      word ptr [0xc49], ax
00BF9D:  C5 16 B7 0B                  lds      dx, ptr [0xbb7]
00BFA1:  B9 00 80                     mov      cx, 0x8000
00BFA4:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
00BFA9:  66 2B C1                     sub      eax, ecx
00BFAC:  79 02                        jns      0xbfb0
00BFAE:  03 C8                        add      cx, ax
00BFB0:  B4 3F                        mov      ah, 0x3f
00BFB2:  CD 21                        int      0x21
00BFB4:  53                           push     bx
00BFB5:  8B C8                        mov      cx, ax
00BFB7:  65 8B 1E 49 0C               mov      bx, word ptr gs:[0xc49]
00BFBC:  B4 40                        mov      ah, 0x40
00BFBE:  CD 21                        int      0x21
00BFC0:  5B                           pop      bx
00BFC1:  65 83 06 A7 0B 02            add      word ptr gs:[0xba7], 2
00BFC7:  66 65 29 0E 92 0A            sub      dword ptr gs:[0xa92], ecx
00BFCD:  75 D2                        jne      0xbfa1
00BFCF:  8B C1                        mov      ax, cx
00BFD1:  8B C8                        mov      cx, ax
00BFD3:  25 FF 3F                     and      ax, 0x3fff
00BFD6:  3B C1                        cmp      ax, cx
00BFD8:  75 05                        jne      0xbfdf
00BFDA:  65 FF 0E A7 0B               dec      word ptr gs:[0xba7]
00BFDF:  65 C7 06 A9 0B 00 40         mov      word ptr gs:[0xba9], 0x4000
00BFE6:  0B C0                        or       ax, ax
00BFE8:  74 04                        je       0xbfee
00BFEA:  65 A3 A9 0B                  mov      word ptr gs:[0xba9], ax
00BFEE:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
00BFF4:  75 04                        jne      0xbffa
00BFF6:  B4 3E                        mov      ah, 0x3e
00BFF8:  CD 21                        int      0x21
00BFFA:  66 5D                        pop      ebp
00BFFC:  5F                           pop      di
00BFFD:  5E                           pop      si
00BFFE:  1F                           pop      ds
00BFFF:  07                           pop      es
00C000:  5A                           pop      dx
00C001:  59                           pop      cx
00C002:  5B                           pop      bx
00C003:  58                           pop      ax
00C004:  CB                           retf    
