; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003ab3
; seg_off: 0299:0b23
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: gfx_clipped_primitive_c
; label_comment: clipped graphics draw primitive (2 calls): same clipped-draw family (dx-param, bp=ax). Shape/span draw variant. The 0x3321/0x32ac/0x39bb/0x3ab3 cluster are the game's 2D drawing primitives
; incoming: call@0x009474->0299:0b23
; byte_count: 146
; boundary: cfg_blocks_13_terminals_2
; terminal: jmp 0x3b3c:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 07aed887cd435f177d260509859785a43910cf689dc0b5d0a8d3a7115b1f280f

003AB3:  50                           push     ax
003AB4:  53                           push     bx
003AB5:  51                           push     cx
003AB6:  52                           push     dx
003AB7:  06                           push     es
003AB8:  1E                           push     ds
003AB9:  57                           push     di
003ABA:  55                           push     bp
003ABB:  0B D2                        or       dx, dx
003ABD:  74 7D                        je       0x3b3c
003ABF:  78 7B                        js       0x3b3c
003AC1:  8B E8                        mov      bp, ax
003AC3:  8C E8                        mov      ax, gs
003AC5:  8E D8                        mov      ds, ax
003AC7:  C4 3E 19 52                  les      di, ptr [0x5219]
003ACB:  3B 1E 35 52                  cmp      bx, word ptr [0x5235]
003ACF:  7C 6B                        jl       0x3b3c
003AD1:  3B 1E 37 52                  cmp      bx, word ptr [0x5237]
003AD5:  7D 65                        jge      0x3b3c
003AD7:  8B C1                        mov      ax, cx
003AD9:  2B 06 39 52                  sub      ax, word ptr [0x5239]
003ADD:  79 0A                        jns      0x3ae9
003ADF:  F7 D8                        neg      ax
003AE1:  2B D0                        sub      dx, ax
003AE3:  7E 57                        jle      0x3b3c
003AE5:  8B 0E 39 52                  mov      cx, word ptr [0x5239]
003AE9:  8B C1                        mov      ax, cx
003AEB:  03 C2                        add      ax, dx
003AED:  2B 06 3B 52                  sub      ax, word ptr [0x523b]
003AF1:  7E 04                        jle      0x3af7
003AF3:  2B D0                        sub      dx, ax
003AF5:  7E 45                        jle      0x3b3c
003AF7:  8B C1                        mov      ax, cx
003AF9:  C1 E0 04                     shl      ax, 4
003AFC:  C1 E1 06                     shl      cx, 6
003AFF:  03 C1                        add      ax, cx
003B01:  8A CB                        mov      cl, bl
003B03:  80 E1 03                     and      cl, 3
003B06:  C1 EB 02                     shr      bx, 2
003B09:  03 C3                        add      ax, bx
003B0B:  03 F8                        add      di, ax
003B0D:  B4 01                        mov      ah, 1
003B0F:  D2 E4                        shl      ah, cl
003B11:  B0 02                        mov      al, 2
003B13:  8B CA                        mov      cx, dx
003B15:  BA C4 03                     mov      dx, 0x3c4
003B18:  EF                           out      dx, ax
003B19:  8B C5                        mov      ax, bp
003B1B:  BA 50 00                     mov      dx, 0x50
003B1E:  F6 06 56 5B 01               test     byte ptr [0x5b56], 1
003B23:  74 10                        je       0x3b35
003B25:  BB 11 5F                     mov      bx, 0x5f11
003B28:  26 8A 05                     mov      al, byte ptr es:[di]
003B2B:  D7                           xlatb   
003B2C:  26 88 05                     mov      byte ptr es:[di], al
003B2F:  03 FA                        add      di, dx
003B31:  E2 F5                        loop     0x3b28
003B33:  EB 07                        jmp      0x3b3c
003B35:  26 88 05                     mov      byte ptr es:[di], al
003B38:  03 FA                        add      di, dx
003B3A:  E2 F9                        loop     0x3b35
003B3C:  5D                           pop      bp
003B3D:  5F                           pop      di
003B3E:  1F                           pop      ds
003B3F:  07                           pop      es
003B40:  5A                           pop      dx
003B41:  59                           pop      cx
003B42:  5B                           pop      bx
003B43:  58                           pop      ax
003B44:  CB                           retf    
