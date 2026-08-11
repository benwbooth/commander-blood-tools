; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003321
; seg_off: 0299:0391
; group: seg_0299
; provenance: recursive_graph
; label: gfx_clipped_draw
; label_comment: clipped graphics draw (3 calls): call 0xbc2; les di,[0x5221] (display buffer); clip bx(x) vs [0x5235]/[0x5237], cx(y) vs [0x5239]; draws with left/top clamp (neg ax; sub dx,ax). A clipped blit/span primitive into the visible page
; byte_count: 125
; boundary: cfg_blocks_13_terminals_2
; terminal: jmp 0x3396:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 3650cb0ea04f103b6c3acce45f65881fde88135096ab4354eae1f24711596417

003321:  50                           push     ax
003322:  51                           push     cx
003323:  52                           push     dx
003324:  06                           push     es
003325:  1E                           push     ds
003326:  57                           push     di
003327:  55                           push     bp
003328:  0B D2                        or       dx, dx
00332A:  74 6A                        je       0x3396
00332C:  78 68                        js       0x3396
00332E:  8B E8                        mov      bp, ax
003330:  8C E8                        mov      ax, gs
003332:  8E D8                        mov      ds, ax
003334:  C4 3E 21 52                  les      di, ptr [0x5221]
003338:  3B 1E 35 52                  cmp      bx, word ptr [0x5235]
00333C:  7C 58                        jl       0x3396
00333E:  3B 1E 37 52                  cmp      bx, word ptr [0x5237]
003342:  7D 52                        jge      0x3396
003344:  8B C1                        mov      ax, cx
003346:  2B 06 39 52                  sub      ax, word ptr [0x5239]
00334A:  79 0A                        jns      0x3356
00334C:  F7 D8                        neg      ax
00334E:  2B D0                        sub      dx, ax
003350:  7E 44                        jle      0x3396
003352:  8B 0E 39 52                  mov      cx, word ptr [0x5239]
003356:  8B C1                        mov      ax, cx
003358:  03 C2                        add      ax, dx
00335A:  2B 06 3B 52                  sub      ax, word ptr [0x523b]
00335E:  7E 04                        jle      0x3364
003360:  2B D0                        sub      dx, ax
003362:  7E 32                        jle      0x3396
003364:  8B C1                        mov      ax, cx
003366:  86 E0                        xchg     al, ah
003368:  C1 E1 06                     shl      cx, 6
00336B:  03 C1                        add      ax, cx
00336D:  03 C3                        add      ax, bx
00336F:  03 F8                        add      di, ax
003371:  8B C5                        mov      ax, bp
003373:  8B CA                        mov      cx, dx
003375:  BA 40 01                     mov      dx, 0x140
003378:  F6 06 56 5B 01               test     byte ptr [0x5b56], 1
00337D:  74 10                        je       0x338f
00337F:  BB 11 5F                     mov      bx, 0x5f11
003382:  26 8A 05                     mov      al, byte ptr es:[di]
003385:  D7                           xlatb   
003386:  26 88 05                     mov      byte ptr es:[di], al
003389:  03 FA                        add      di, dx
00338B:  E2 F5                        loop     0x3382
00338D:  EB 07                        jmp      0x3396
00338F:  26 88 05                     mov      byte ptr es:[di], al
003392:  03 FA                        add      di, dx
003394:  E2 F9                        loop     0x338f
003396:  5D                           pop      bp
003397:  5F                           pop      di
003398:  1F                           pop      ds
003399:  07                           pop      es
00339A:  5A                           pop      dx
00339B:  59                           pop      cx
00339C:  58                           pop      ax
00339D:  CB                           retf    
