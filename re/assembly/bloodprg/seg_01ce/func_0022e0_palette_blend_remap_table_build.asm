; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0022e0
; seg_off: 01ce:0000
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: palette_blend_remap_table_build
; label_comment: CORRECTED (was abs_negate_gs_setup, which only described the first four instructions). THE TINT REMAP-TABLE BUILDER, far-called as 0x1CE:0x0000 with DI = a 256-byte destination table, AX = the NEGATED blend percent, BX/CX/DX = the target colour. Prescales the target (pct*comp/100, three pushes at 0x22F5/0x22FD/0x2304) and the source weight (100-pct @0x230D); then for each of the 256 entries of the LIVE palette at DS:0x5251 computes blended = src*(100-pct)/100 + target*pct/100 and NEAREST-SEARCHES all 256 entries by squared RGB distance (0x2366..0x238E), seed distance 0xBB8 and `ja` on strictly-greater so TIES TAKE THE LATER index; a source with no match within 0xBB8 leaves its table byte UNCHANGED (`js 0x23B0` skips the store). This is how every translucent/tinted overlay in the game is drawn. PORTED: palette.rs build_palette_blend_remap_table || SUPERSEDED READING `abs_negate_gs_setup`: abs/negate helper: bp=gs; ds=es=gs; si=dx; neg ax. Rebases segments to the work arena and negates/absolutes the ax argument (called 5x) || MERGED 2026-07-25 (audit-fixes #131): the superseded rows were left in the file beside their own correction, so a reader who found one had no pointer to the other.
; incoming: call@0x0090f9->01ce:0000
; incoming: call@0x00961d->01ce:0000
; incoming: call@0x009e9b->01ce:0000
; incoming: call@0x00b1b3->01ce:0000
; incoming: call@0x00b47c->01ce:0000
; byte_count: 229
; boundary: cfg_blocks_17_terminals_3
; terminal: jmp 0x231f:1, jmp 0x2357:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ffc21fec39315ead7e4c402542613eaadc680af73b2eff8e39d531d5e23b13c3

0022E0:  55                           push     bp
0022E1:  50                           push     ax
0022E2:  53                           push     bx
0022E3:  51                           push     cx
0022E4:  52                           push     dx
0022E5:  06                           push     es
0022E6:  57                           push     di
0022E7:  1E                           push     ds
0022E8:  56                           push     si
0022E9:  8C ED                        mov      bp, gs
0022EB:  8E DD                        mov      ds, bp
0022ED:  8E C5                        mov      es, bp
0022EF:  8B F2                        mov      si, dx
0022F1:  F7 D8                        neg      ax
0022F3:  8B E8                        mov      bp, ax
0022F5:  F7 E3                        mul      bx
0022F7:  BB 64 00                     mov      bx, 0x64
0022FA:  F7 F3                        div      bx
0022FC:  50                           push     ax
0022FD:  8B C5                        mov      ax, bp
0022FF:  F7 E1                        mul      cx
002301:  F7 F3                        div      bx
002303:  50                           push     ax
002304:  8B C5                        mov      ax, bp
002306:  8B D6                        mov      dx, si
002308:  F7 E2                        mul      dx
00230A:  F7 F3                        div      bx
00230C:  50                           push     ax
00230D:  8B C5                        mov      ax, bp
00230F:  83 E8 64                     sub      ax, 0x64
002312:  F7 D8                        neg      ax
002314:  50                           push     ax
002315:  BE 51 52                     mov      si, 0x5251
002318:  83 EC 10                     sub      sp, 0x10
00231B:  8B EC                        mov      bp, sp
00231D:  33 C9                        xor      cx, cx
00231F:  BB 64 00                     mov      bx, 0x64
002322:  AC                           lodsb    al, byte ptr [si]
002323:  98                           cwde    
002324:  F7 66 10                     mul      word ptr [bp + 0x10]
002327:  F7 F3                        div      bx
002329:  03 46 16                     add      ax, word ptr [bp + 0x16]
00232C:  89 46 0E                     mov      word ptr [bp + 0xe], ax
00232F:  AC                           lodsb    al, byte ptr [si]
002330:  98                           cwde    
002331:  F7 66 10                     mul      word ptr [bp + 0x10]
002334:  F7 F3                        div      bx
002336:  03 46 14                     add      ax, word ptr [bp + 0x14]
002339:  89 46 0C                     mov      word ptr [bp + 0xc], ax
00233C:  AC                           lodsb    al, byte ptr [si]
00233D:  98                           cwde    
00233E:  F7 66 10                     mul      word ptr [bp + 0x10]
002341:  F7 F3                        div      bx
002343:  03 46 12                     add      ax, word ptr [bp + 0x12]
002346:  89 46 0A                     mov      word ptr [bp + 0xa], ax
002349:  56                           push     si
00234A:  C7 46 08 FF FF               mov      word ptr [bp + 8], 0xffff
00234F:  C7 46 06 B8 0B               mov      word ptr [bp + 6], 0xbb8
002354:  BE 51 52                     mov      si, 0x5251
002357:  AC                           lodsb    al, byte ptr [si]
002358:  98                           cwde    
002359:  89 46 04                     mov      word ptr [bp + 4], ax
00235C:  AC                           lodsb    al, byte ptr [si]
00235D:  98                           cwde    
00235E:  89 46 02                     mov      word ptr [bp + 2], ax
002361:  AC                           lodsb    al, byte ptr [si]
002362:  98                           cwde    
002363:  89 46 00                     mov      word ptr [bp], ax
002366:  8B 46 0E                     mov      ax, word ptr [bp + 0xe]
002369:  2B 46 04                     sub      ax, word ptr [bp + 4]
00236C:  79 02                        jns      0x2370
00236E:  F7 D8                        neg      ax
002370:  F7 E0                        mul      ax
002372:  8B D8                        mov      bx, ax
002374:  8B 46 0C                     mov      ax, word ptr [bp + 0xc]
002377:  2B 46 02                     sub      ax, word ptr [bp + 2]
00237A:  79 02                        jns      0x237e
00237C:  F7 D8                        neg      ax
00237E:  F7 E0                        mul      ax
002380:  03 D8                        add      bx, ax
002382:  8B 46 0A                     mov      ax, word ptr [bp + 0xa]
002385:  2B 46 00                     sub      ax, word ptr [bp]
002388:  79 02                        jns      0x238c
00238A:  F7 D8                        neg      ax
00238C:  F7 E0                        mul      ax
00238E:  03 D8                        add      bx, ax
002390:  3B 5E 06                     cmp      bx, word ptr [bp + 6]
002393:  77 0A                        ja       0x239f
002395:  89 5E 06                     mov      word ptr [bp + 6], bx
002398:  88 4E 08                     mov      byte ptr [bp + 8], cl
00239B:  C6 46 09 00                  mov      byte ptr [bp + 9], 0
00239F:  FE C1                        inc      cl
0023A1:  74 02                        je       0x23a5
0023A3:  EB B2                        jmp      0x2357
0023A5:  5E                           pop      si
0023A6:  8B 46 08                     mov      ax, word ptr [bp + 8]
0023A9:  0B C0                        or       ax, ax
0023AB:  78 03                        js       0x23b0
0023AD:  26 88 05                     mov      byte ptr es:[di], al
0023B0:  47                           inc      di
0023B1:  FE C5                        inc      ch
0023B3:  74 03                        je       0x23b8
0023B5:  E9 67 FF                     jmp      0x231f
0023B8:  83 C4 18                     add      sp, 0x18
0023BB:  5E                           pop      si
0023BC:  1F                           pop      ds
0023BD:  5F                           pop      di
0023BE:  07                           pop      es
0023BF:  5A                           pop      dx
0023C0:  59                           pop      cx
0023C1:  5B                           pop      bx
0023C2:  58                           pop      ax
0023C3:  5D                           pop      bp
0023C4:  CB                           retf    
