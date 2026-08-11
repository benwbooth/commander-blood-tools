; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0039bb
; seg_off: 0299:0a2b
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: gfx_clipped_span_fill
; label_comment: clipped horizontal-span/rect fill: bx=x cx=y dx=width bp=color/pattern; clip bounds gs:0x5235(left)/0x5237(right)/0x5239(top)/0x523b(bottom); VGA plane-mask setup at 0x3a23. The concept-box backdrop primitive || ALSO RECORDED as `gfx_clipped_primitive_b`: clipped graphics draw primitive (2 calls): same dx-param + bp=ax + clip family as 0x3321/0x32ac. Another shape/span draw variant || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; incoming: call@0x00946d->0299:0a2b
; byte_count: 248
; boundary: cfg_blocks_27_terminals_3
; terminal: jmp 0x3a9e:1, jmp 0x3aaa:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 462a918d52d2c1f7d3741acf183ddd6fcca6f2a3ab78243a1d7c8fa3ab9f6630

0039BB:  50                           push     ax
0039BC:  53                           push     bx
0039BD:  51                           push     cx
0039BE:  52                           push     dx
0039BF:  1E                           push     ds
0039C0:  06                           push     es
0039C1:  57                           push     di
0039C2:  55                           push     bp
0039C3:  0B D2                        or       dx, dx
0039C5:  0F 84 E1 00                  je       0x3aaa
0039C9:  0F 88 DD 00                  js       0x3aaa
0039CD:  8B E8                        mov      bp, ax
0039CF:  8C E8                        mov      ax, gs
0039D1:  8E D8                        mov      ds, ax
0039D3:  C4 3E 19 52                  les      di, ptr [0x5219]
0039D7:  3B 0E 39 52                  cmp      cx, word ptr [0x5239]
0039DB:  0F 8C CB 00                  jl       0x3aaa
0039DF:  3B 0E 3B 52                  cmp      cx, word ptr [0x523b]
0039E3:  0F 8D C3 00                  jge      0x3aaa
0039E7:  8B C3                        mov      ax, bx
0039E9:  2B 06 35 52                  sub      ax, word ptr [0x5235]
0039ED:  79 0C                        jns      0x39fb
0039EF:  F7 D8                        neg      ax
0039F1:  2B D0                        sub      dx, ax
0039F3:  0F 8E B3 00                  jle      0x3aaa
0039F7:  8B 1E 35 52                  mov      bx, word ptr [0x5235]
0039FB:  8B C3                        mov      ax, bx
0039FD:  03 C2                        add      ax, dx
0039FF:  2B 06 37 52                  sub      ax, word ptr [0x5237]
003A03:  7C 06                        jl       0x3a0b
003A05:  2B D0                        sub      dx, ax
003A07:  0F 8E 9F 00                  jle      0x3aaa
003A0B:  8B C1                        mov      ax, cx
003A0D:  C1 E0 04                     shl      ax, 4
003A10:  C1 E1 06                     shl      cx, 6
003A13:  03 C1                        add      ax, cx
003A15:  8A CB                        mov      cl, bl
003A17:  83 E1 03                     and      cx, 3
003A1A:  C1 EB 02                     shr      bx, 2
003A1D:  03 C3                        add      ax, bx
003A1F:  03 F8                        add      di, ax
003A21:  8B DA                        mov      bx, dx
003A23:  BA C4 03                     mov      dx, 0x3c4
003A26:  B0 02                        mov      al, 2
003A28:  EE                           out      dx, al
003A29:  42                           inc      dx
003A2A:  8B C5                        mov      ax, bp
003A2C:  8A E0                        mov      ah, al
003A2E:  F6 06 56 5B 01               test     byte ptr [0x5b56], 1
003A33:  74 37                        je       0x3a6c
003A35:  B0 11                        mov      al, 0x11
003A37:  D2 E0                        shl      al, cl
003A39:  8A CB                        mov      cl, bl
003A3B:  C1 EB 02                     shr      bx, 2
003A3E:  B5 04                        mov      ch, 4
003A40:  53                           push     bx
003A41:  FE C1                        inc      cl
003A43:  80 E1 03                     and      cl, 3
003A46:  75 01                        jne      0x3a49
003A48:  43                           inc      bx
003A49:  FE CD                        dec      ch
003A4B:  75 F3                        jne      0x3a40
003A4D:  BB 11 5F                     mov      bx, 0x5f11
003A50:  B4 04                        mov      ah, 4
003A52:  EE                           out      dx, al
003A53:  59                           pop      cx
003A54:  E3 0B                        jcxz     0x3a61
003A56:  50                           push     ax
003A57:  57                           push     di
003A58:  26 8A 05                     mov      al, byte ptr es:[di]
003A5B:  D7                           xlatb   
003A5C:  AA                           stosb    byte ptr es:[di], al
003A5D:  E2 F9                        loop     0x3a58
003A5F:  5F                           pop      di
003A60:  58                           pop      ax
003A61:  D0 C0                        rol      al, 1
003A63:  83 D7 00                     adc      di, 0
003A66:  FE CC                        dec      ah
003A68:  75 E8                        jne      0x3a52
003A6A:  EB 3E                        jmp      0x3aaa
003A6C:  E3 18                        jcxz     0x3a86
003A6E:  B0 0F                        mov      al, 0xf
003A70:  D2 E0                        shl      al, cl
003A72:  80 F1 03                     xor      cl, 3
003A75:  FE C1                        inc      cl
003A77:  2B D9                        sub      bx, cx
003A79:  79 06                        jns      0x3a81
003A7B:  F7 DB                        neg      bx
003A7D:  24 0F                        and      al, 0xf
003A7F:  EB 1D                        jmp      0x3a9e
003A81:  EE                           out      dx, al
003A82:  26 88 25                     mov      byte ptr es:[di], ah
003A85:  47                           inc      di
003A86:  B0 0F                        mov      al, 0xf
003A88:  8B CB                        mov      cx, bx
003A8A:  C1 E9 02                     shr      cx, 2
003A8D:  EE                           out      dx, al
003A8E:  8A C4                        mov      al, ah
003A90:  F3 AA                        rep stosb byte ptr es:[di], al
003A92:  80 E3 03                     and      bl, 3
003A95:  74 13                        je       0x3aaa
003A97:  B0 0F                        mov      al, 0xf
003A99:  80 F3 03                     xor      bl, 3
003A9C:  FE C3                        inc      bl
003A9E:  8A CB                        mov      cl, bl
003AA0:  8A D8                        mov      bl, al
003AA2:  D2 E8                        shr      al, cl
003AA4:  22 C3                        and      al, bl
003AA6:  EE                           out      dx, al
003AA7:  26 88 25                     mov      byte ptr es:[di], ah
003AAA:  5D                           pop      bp
003AAB:  5F                           pop      di
003AAC:  07                           pop      es
003AAD:  1F                           pop      ds
003AAE:  5A                           pop      dx
003AAF:  59                           pop      cx
003AB0:  5B                           pop      bx
003AB1:  58                           pop      ax
003AB2:  CB                           retf    
