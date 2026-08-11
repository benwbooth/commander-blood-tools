; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003b85
; seg_off: 0299:0bf5
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: blit_coord_guard_b
; label_comment: blit coordinate guard (sibling of 0x339e): or dx,dx; je/js 0x3c63 cull; or bp,bp. Off-screen/degenerate-span cull
; incoming: call@0x007a94->0299:0bf5
; incoming: call@0x007b18->0299:0bf5
; byte_count: 231
; boundary: cfg_blocks_31_terminals_2
; terminal: jmp 0x3c63:1, retf:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_0299/func_003b85_blit_coord_guard_b.cpp
; routine_bytes_sha256: f20fddb25b109931a6b96df8fac268ae1f3beb6f50889d4e94295ce92a8ec89b

003B85:  50                           push     ax
003B86:  53                           push     bx
003B87:  51                           push     cx
003B88:  52                           push     dx
003B89:  55                           push     bp
003B8A:  06                           push     es
003B8B:  57                           push     di
003B8C:  56                           push     si
003B8D:  0B D2                        or       dx, dx
003B8F:  0F 84 D0 00                  je       0x3c63
003B93:  0F 88 CC 00                  js       0x3c63
003B97:  0B ED                        or       bp, bp
003B99:  0F 84 C6 00                  je       0x3c63
003B9D:  0F 88 C2 00                  js       0x3c63
003BA1:  8B F0                        mov      si, ax
003BA3:  8B C3                        mov      ax, bx
003BA5:  2B 06 35 52                  sub      ax, word ptr [0x5235]
003BA9:  79 0E                        jns      0x3bb9
003BAB:  03 D0                        add      dx, ax
003BAD:  0F 88 B2 00                  js       0x3c63
003BB1:  0F 84 AE 00                  je       0x3c63
003BB5:  8B 1E 35 52                  mov      bx, word ptr [0x5235]
003BB9:  8B C3                        mov      ax, bx
003BBB:  03 C2                        add      ax, dx
003BBD:  2B 06 37 52                  sub      ax, word ptr [0x5237]
003BC1:  78 0A                        js       0x3bcd
003BC3:  2B D0                        sub      dx, ax
003BC5:  0F 88 9A 00                  js       0x3c63
003BC9:  0F 84 96 00                  je       0x3c63
003BCD:  8B C1                        mov      ax, cx
003BCF:  2B 06 39 52                  sub      ax, word ptr [0x5239]
003BD3:  79 0E                        jns      0x3be3
003BD5:  03 E8                        add      bp, ax
003BD7:  0F 88 88 00                  js       0x3c63
003BDB:  0F 84 84 00                  je       0x3c63
003BDF:  8B 0E 39 52                  mov      cx, word ptr [0x5239]
003BE3:  8B C1                        mov      ax, cx
003BE5:  03 C5                        add      ax, bp
003BE7:  2B 06 37 52                  sub      ax, word ptr [0x5237]
003BEB:  78 06                        js       0x3bf3
003BED:  2B E8                        sub      bp, ax
003BEF:  78 72                        js       0x3c63
003BF1:  74 70                        je       0x3c63
003BF3:  C4 3E 21 52                  les      di, ptr [0x5221]
003BF7:  8B C1                        mov      ax, cx
003BF9:  86 C4                        xchg     ah, al
003BFB:  C1 E1 06                     shl      cx, 6
003BFE:  03 C1                        add      ax, cx
003C00:  03 C3                        add      ax, bx
003C02:  03 F8                        add      di, ax
003C04:  8B CD                        mov      cx, bp
003C06:  BD 40 01                     mov      bp, 0x140
003C09:  2B EA                        sub      bp, dx
003C0B:  B8 FF FF                     mov      ax, 0xffff
003C0E:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
003C13:  52                           push     dx
003C14:  BA EF 00                     mov      dx, 0xef
003C17:  B3 10                        mov      bl, 0x10
003C19:  4E                           dec      si
003C1A:  74 05                        je       0x3c21
003C1C:  86 D6                        xchg     dh, dl
003C1E:  4E                           dec      si
003C1F:  75 22                        jne      0x3c43
003C21:  5E                           pop      si
003C22:  51                           push     cx
003C23:  8B CE                        mov      cx, si
003C25:  D1 D0                        rcl      ax, 1
003C27:  73 03                        jae      0x3c2c
003C29:  26 88 15                     mov      byte ptr es:[di], dl
003C2C:  47                           inc      di
003C2D:  FE CB                        dec      bl
003C2F:  75 09                        jne      0x3c3a
003C31:  8B D8                        mov      bx, ax
003C33:  C1 D0 04                     rcl      ax, 4
003C36:  33 C3                        xor      ax, bx
003C38:  B3 10                        mov      bl, 0x10
003C3A:  E2 E9                        loop     0x3c25
003C3C:  03 FD                        add      di, bp
003C3E:  59                           pop      cx
003C3F:  E2 E1                        loop     0x3c22
003C41:  EB 20                        jmp      0x3c63
003C43:  92                           xchg     dx, ax
003C44:  5E                           pop      si
003C45:  51                           push     cx
003C46:  8B CE                        mov      cx, si
003C48:  D1 D2                        rcl      dx, 1
003C4A:  73 02                        jae      0x3c4e
003C4C:  86 C4                        xchg     ah, al
003C4E:  AA                           stosb    byte ptr es:[di], al
003C4F:  FE CB                        dec      bl
003C51:  75 09                        jne      0x3c5c
003C53:  8B DA                        mov      bx, dx
003C55:  C1 D2 03                     rcl      dx, 3
003C58:  33 D3                        xor      dx, bx
003C5A:  B3 10                        mov      bl, 0x10
003C5C:  E2 EA                        loop     0x3c48
003C5E:  03 FD                        add      di, bp
003C60:  59                           pop      cx
003C61:  E2 E2                        loop     0x3c45
003C63:  5E                           pop      si
003C64:  5F                           pop      di
003C65:  07                           pop      es
003C66:  5D                           pop      bp
003C67:  5A                           pop      dx
003C68:  59                           pop      cx
003C69:  5B                           pop      bx
003C6A:  58                           pop      ax
003C6B:  CB                           retf    
