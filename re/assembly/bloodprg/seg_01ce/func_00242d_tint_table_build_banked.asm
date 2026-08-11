; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00242d
; seg_off: 01ce:014d
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: tint_table_build_banked
; label_comment: THE SECOND TABLE BUILDER (far 0x1CE:0x014D; the first is 0x22E0 = 0x1CE:0x0000). AX = a parameter, BX = the destination table, and it walks the LIVE PALETTE DS:0x5251 for cx=0x100 entries. Called once, at 0x9622, with ax=0xE0 (224) and bx=DS:0x6011 -- which is why 0x6011 is all-zero in the image and never appears as a `mov di` destination for 0x22E0: a different builder fills it. 224 is the base of the 16-colour CONSOLE BANK (224..239) the intro band's pixels all lie in, so this table plausibly remaps the screen INTO that bank; the montage's full-screen remap at 0x7ACE uses exactly this table. ALREADY LIFTED BIT-EXACTLY: recomp func_242d (oracle-verified) || SUPERSEDED READING `ds_es_rebase_gs`: helper prologue: bp=ax; ds=es=gs. Rebases the segment registers to the GS work arena before a data operation || MERGED 2026-07-25 (audit-fixes #131): the superseded rows were left in the file beside their own correction, so a reader who found one had no pointer to the other.
; incoming: call@0x009628->01ce:014d
; byte_count: 94
; boundary: cfg_blocks_6_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: fa951f30b87b00b6bcd2badbb4fc0c8bc664233194e9cc84ac8d6258eb97c869

00242D:  51                           push     cx
00242E:  52                           push     dx
00242F:  06                           push     es
002430:  57                           push     di
002431:  1E                           push     ds
002432:  56                           push     si
002433:  55                           push     bp
002434:  8B E8                        mov      bp, ax
002436:  8C E8                        mov      ax, gs
002438:  8E D8                        mov      ds, ax
00243A:  8E C0                        mov      es, ax
00243C:  8B FB                        mov      di, bx
00243E:  BE 51 52                     mov      si, 0x5251
002441:  B9 00 01                     mov      cx, 0x100
002444:  32 E4                        xor      ah, ah
002446:  33 DB                        xor      bx, bx
002448:  51                           push     cx
002449:  AC                           lodsb    al, byte ptr [si]
00244A:  BA 03 00                     mov      dx, 3
00244D:  F6 E2                        mul      dl
00244F:  8B C8                        mov      cx, ax
002451:  AC                           lodsb    al, byte ptr [si]
002452:  BA 06 00                     mov      dx, 6
002455:  F6 E2                        mul      dl
002457:  03 C8                        add      cx, ax
002459:  AC                           lodsb    al, byte ptr [si]
00245A:  98                           cwde    
00245B:  03 C8                        add      cx, ax
00245D:  8B C1                        mov      ax, cx
00245F:  B9 1C 00                     mov      cx, 0x1c
002462:  F6 F1                        div      cl
002464:  98                           cwde    
002465:  83 F8 0F                     cmp      ax, 0xf
002468:  7E 03                        jle      0x246d
00246A:  B8 0F 00                     mov      ax, 0xf
00246D:  03 C5                        add      ax, bp
00246F:  3B DD                        cmp      bx, bp
002471:  7C 0B                        jl       0x247e
002473:  8B D5                        mov      dx, bp
002475:  83 C2 0F                     add      dx, 0xf
002478:  3B DA                        cmp      bx, dx
00247A:  7F 02                        jg       0x247e
00247C:  8A C3                        mov      al, bl
00247E:  AA                           stosb    byte ptr es:[di], al
00247F:  59                           pop      cx
002480:  43                           inc      bx
002481:  E2 C5                        loop     0x2448
002483:  5D                           pop      bp
002484:  5E                           pop      si
002485:  1F                           pop      ds
002486:  5F                           pop      di
002487:  07                           pop      es
002488:  5A                           pop      dx
002489:  59                           pop      cx
00248A:  CB                           retf    
