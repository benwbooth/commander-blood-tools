; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0072a8
; seg_off: 04da:1f08
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: dlg_menu_words_inline_reveal_step
; label_comment: redraws the visible prefix of the current concept-menu word list from DS:[0x674a] through the unsigned GS:0x27d3 reveal boundary. Each word is drawn at x=GS:0x27d1/y=8 with color 0xef; punctuation removes the six-pixel gap, and a signed x+next-width comparison wraps at 300. When GS:0x0b35 reaches zero it advances the boundary by one word and reloads the selected delay at GS:0x0aca. A 0/0xffff word instead sets the final hold to GS:0x27cf*(GS:0x0aca>>1)+6 and raises GS:0x67bb, subject to the presentation gates.
; incoming: call@0x0012b8->04da:1f08
; byte_count: 243
; boundary: cfg_blocks_21_terminals_3
; terminal: jmp 0x7349:1, jmp 0x7392:1, retf:1
; direct_callees: none
; indirect_calls: 2
; routine_bytes_sha256: 894d53441a36ff524fc93c8b3785a259c65a9eddd978f94ed81b57b1eb1ec500

0072A8:  06                           push     es
0072A9:  57                           push     di
0072AA:  1E                           push     ds
0072AB:  56                           push     si
0072AC:  50                           push     ax
0072AD:  53                           push     bx
0072AE:  51                           push     cx
0072AF:  52                           push     dx
0072B0:  F6 06 B0 67 01               test     byte ptr [0x67b0], 1
0072B5:  75 13                        jne      0x72ca
0072B7:  F6 06 BC 67 01               test     byte ptr [0x67bc], 1
0072BC:  0F 84 D2 00                  je       0x7392
0072C0:  81 3E 9A 67 B0 67            cmp      word ptr [0x679a], 0x67b0
0072C6:  0F 85 C8 00                  jne      0x7392
0072CA:  C7 06 D1 27 0A 00            mov      word ptr [0x27d1], 0xa
0072D0:  66 33 C0                     xor      eax, eax
0072D3:  C4 3E 4A 67                  les      di, ptr [0x674a]
0072D7:  BA 08 00                     mov      dx, 8
0072DA:  65 C5 36 28 67               lds      si, ptr gs:[0x6728]
0072DF:  26 8B 05                     mov      ax, word ptr es:[di]
0072E2:  0B C0                        or       ax, ax
0072E4:  0F 84 80 00                  je       0x7368
0072E8:  83 F8 FF                     cmp      ax, -1
0072EB:  74 7B                        je       0x7368
0072ED:  03 F0                        add      si, ax
0072EF:  B0 EF                        mov      al, 0xef
0072F1:  65 8B 1E D1 27               mov      bx, word ptr gs:[0x27d1]
0072F6:  9A DE 05 99 02               lcall    0x299, 0x5de
0072FB:  65 8B 1E CD 27               mov      bx, word ptr gs:[0x27cd]
007300:  83 C7 02                     add      di, 2
007303:  26 8B 35                     mov      si, word ptr es:[di]
007306:  8A 04                        mov      al, byte ptr [si]
007308:  3C 2E                        cmp      al, 0x2e
00730A:  74 10                        je       0x731c
00730C:  3C 2C                        cmp      al, 0x2c
00730E:  74 0C                        je       0x731c
007310:  3C 3A                        cmp      al, 0x3a
007312:  74 08                        je       0x731c
007314:  3C 21                        cmp      al, 0x21
007316:  74 04                        je       0x731c
007318:  3C 3F                        cmp      al, 0x3f
00731A:  75 07                        jne      0x7323
00731C:  65 01 1E D1 27               add      word ptr gs:[0x27d1], bx
007321:  EB 26                        jmp      0x7349
007323:  83 C3 06                     add      bx, 6
007326:  B8 01 00                     mov      ax, 1
007329:  9A 3D 01 99 02               lcall    0x299, 0x13d
00732E:  65 01 1E D1 27               add      word ptr gs:[0x27d1], bx
007333:  65 8B 1E D1 27               mov      bx, word ptr gs:[0x27d1]
007338:  03 C3                        add      ax, bx
00733A:  3D 2C 01                     cmp      ax, 0x12c
00733D:  7C 0A                        jl       0x7349
00733F:  65 C7 06 D1 27 0A 00         mov      word ptr gs:[0x27d1], 0xa
007346:  83 C2 08                     add      dx, 8
007349:  65 3B 3E D3 27               cmp      di, word ptr gs:[0x27d3]
00734E:  72 8A                        jb       0x72da
007350:  65 A1 35 0B                  mov      ax, word ptr gs:[0xb35]
007354:  0B C0                        or       ax, ax
007356:  75 3A                        jne      0x7392
007358:  65 83 06 D3 27 02            add      word ptr gs:[0x27d3], 2
00735E:  65 A1 CA 0A                  mov      ax, word ptr gs:[0xaca]
007362:  65 A3 35 0B                  mov      word ptr gs:[0xb35], ax
007366:  EB 2A                        jmp      0x7392
007368:  65 F6 06 BC 67 01            test     byte ptr gs:[0x67bc], 1
00736E:  75 22                        jne      0x7392
007370:  65 F6 06 BB 67 01            test     byte ptr gs:[0x67bb], 1
007376:  75 1A                        jne      0x7392
007378:  65 A1 CF 27                  mov      ax, word ptr gs:[0x27cf]
00737C:  65 8B 16 CA 0A               mov      dx, word ptr gs:[0xaca]
007381:  D1 EA                        shr      dx, 1
007383:  F7 E2                        mul      dx
007385:  83 C0 06                     add      ax, 6
007388:  65 A3 35 0B                  mov      word ptr gs:[0xb35], ax
00738C:  65 C6 06 BB 67 01            mov      byte ptr gs:[0x67bb], 1
007392:  5A                           pop      dx
007393:  59                           pop      cx
007394:  5B                           pop      bx
007395:  58                           pop      ax
007396:  5E                           pop      si
007397:  1F                           pop      ds
007398:  5F                           pop      di
007399:  07                           pop      es
00739A:  CB                           retf    
