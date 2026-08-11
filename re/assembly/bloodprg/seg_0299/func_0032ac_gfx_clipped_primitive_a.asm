; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0032ac
; seg_off: 0299:031c
; group: seg_0299
; provenance: recursive_graph
; label: gfx_clipped_primitive_a
; label_comment: clipped graphics draw primitive (3 calls): dx-param (or dx,dx; sign check), bp=ax coordinate; same clipped-draw family as gfx_clipped_draw 0x3321. A shape/span draw into the display page with clipping
; byte_count: 117
; boundary: cfg_blocks_13_terminals_2
; terminal: jmp 0x3318:1, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_0032ac_gfx_clipped_primitive_a.cpp
; routine_bytes_sha256: a87fa6100f182d90eaed1ca7299a34c3b6cc995278c15f0d181631fbcb0512eb

0032AC:  50                           push     ax
0032AD:  53                           push     bx
0032AE:  51                           push     cx
0032AF:  52                           push     dx
0032B0:  1E                           push     ds
0032B1:  06                           push     es
0032B2:  57                           push     di
0032B3:  55                           push     bp
0032B4:  0B D2                        or       dx, dx
0032B6:  74 60                        je       0x3318
0032B8:  78 5E                        js       0x3318
0032BA:  8B E8                        mov      bp, ax
0032BC:  8C E8                        mov      ax, gs
0032BE:  8E D8                        mov      ds, ax
0032C0:  C4 3E 21 52                  les      di, ptr [0x5221]
0032C4:  3B 0E 39 52                  cmp      cx, word ptr [0x5239]
0032C8:  7C 4E                        jl       0x3318
0032CA:  3B 0E 3B 52                  cmp      cx, word ptr [0x523b]
0032CE:  7D 48                        jge      0x3318
0032D0:  8B C3                        mov      ax, bx
0032D2:  2B 06 35 52                  sub      ax, word ptr [0x5235]
0032D6:  79 0A                        jns      0x32e2
0032D8:  F7 D8                        neg      ax
0032DA:  2B D0                        sub      dx, ax
0032DC:  7E 3A                        jle      0x3318
0032DE:  8B 1E 35 52                  mov      bx, word ptr [0x5235]
0032E2:  8B C3                        mov      ax, bx
0032E4:  03 C2                        add      ax, dx
0032E6:  2B 06 37 52                  sub      ax, word ptr [0x5237]
0032EA:  7C 04                        jl       0x32f0
0032EC:  2B D0                        sub      dx, ax
0032EE:  7E 28                        jle      0x3318
0032F0:  8B C1                        mov      ax, cx
0032F2:  86 C4                        xchg     ah, al
0032F4:  C1 E1 06                     shl      cx, 6
0032F7:  03 C1                        add      ax, cx
0032F9:  03 C3                        add      ax, bx
0032FB:  03 F8                        add      di, ax
0032FD:  F6 06 56 5B 01               test     byte ptr [0x5b56], 1
003302:  74 0E                        je       0x3312
003304:  8B CA                        mov      cx, dx
003306:  BB 11 5F                     mov      bx, 0x5f11
003309:  26 8A 05                     mov      al, byte ptr es:[di]
00330C:  D7                           xlatb   
00330D:  AA                           stosb    byte ptr es:[di], al
00330E:  E2 F9                        loop     0x3309
003310:  EB 06                        jmp      0x3318
003312:  8B CA                        mov      cx, dx
003314:  8B C5                        mov      ax, bp
003316:  F3 AA                        rep stosb byte ptr es:[di], al
003318:  5D                           pop      bp
003319:  5F                           pop      di
00331A:  07                           pop      es
00331B:  1F                           pop      ds
00331C:  5A                           pop      dx
00331D:  59                           pop      cx
00331E:  5B                           pop      bx
00331F:  58                           pop      ax
003320:  CB                           retf    
