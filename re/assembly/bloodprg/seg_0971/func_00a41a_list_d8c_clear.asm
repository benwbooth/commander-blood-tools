; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a41a
; seg_off: 0971:070a
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_active_present
; label_comment: Retires the active segment from GS:0x0D96 into GS:0x0DAA, parses its rectangle metadata, and selects direct display drawing, back-buffer drawing plus presentation, or the compressed rectangular decoder.
; byte_count: 211
; boundary: cfg_blocks_20_terminals_3
; terminal: jmp 0xa4cf:1, jmp 0xa4e4:1, retf:1
; direct_callees: 0x003e46, 0x00a4ed, 0x00ab25
; indirect_calls: 0
; routine_bytes_sha256: d33eeda97f2b75f7d3446bce71daad29b5c669d95e5958ef7e178d11cf99eeab

00A41A:  55                           push     bp
00A41B:  53                           push     bx
00A41C:  51                           push     cx
00A41D:  52                           push     dx
00A41E:  1E                           push     ds
00A41F:  56                           push     si
00A420:  06                           push     es
00A421:  57                           push     di
00A422:  33 ED                        xor      bp, bp
00A424:  65 87 2E 96 0D               xchg     word ptr gs:[0xd96], bp
00A429:  65 89 2E AA 0D               mov      word ptr gs:[0xdaa], bp
00A42E:  0B ED                        or       bp, bp
00A430:  0F 84 B0 00                  je       0xa4e4
00A434:  65 8B 36 94 0D               mov      si, word ptr gs:[0xd94]
00A439:  1E                           push     ds
00A43A:  65 8E 06 23 52               mov      es, word ptr gs:[0x5223]
00A43F:  65 8B 1E 80 0D               mov      bx, word ptr gs:[0xd80]
00A444:  8E DD                        mov      ds, bp
00A446:  AD                           lodsw    ax, word ptr [si]
00A447:  80 E4 F9                     and      ah, 0xf9
00A44A:  8B F8                        mov      di, ax
00A44C:  AD                           lodsw    ax, word ptr [si]
00A44D:  8B C8                        mov      cx, ax
00A44F:  33 D2                        xor      dx, dx
00A451:  8B C2                        mov      ax, dx
00A453:  53                           push     bx
00A454:  65 F7 06 A4 0D 00 04         test     word ptr gs:[0xda4], 0x400
00A45B:  75 04                        jne      0xa461
00A45D:  AD                           lodsw    ax, word ptr [si]
00A45E:  8B D0                        mov      dx, ax
00A460:  AD                           lodsw    ax, word ptr [si]
00A461:  8B D8                        mov      bx, ax
00A463:  58                           pop      ax
00A464:  65 03 1E A7 1F               add      bx, word ptr gs:[0x1fa7]
00A469:  65 C6 06 B8 0D 01            mov      byte ptr gs:[0xdb8], 1
00A46F:  65 F6 06 B9 0D 01            test     byte ptr gs:[0xdb9], 1
00A475:  74 24                        je       0xa49b
00A477:  0A C9                        or       cl, cl
00A479:  74 10                        je       0xa48b
00A47B:  80 F9 82                     cmp      cl, 0x82
00A47E:  76 02                        jbe      0xa482
00A480:  B1 82                        mov      cl, 0x82
00A482:  66 65 8E 06 2B 52            mov      es, word ptr gs:[0x522b]
00A488:  E8 62 00                     call     0xa4ed
00A48B:  1E                           push     ds
00A48C:  56                           push     si
00A48D:  65 C5 36 29 52               lds      si, ptr gs:[0x5229]
00A492:  9A B6 0E 99 02               lcall    0x299, 0xeb6
00A497:  5E                           pop      si
00A498:  1F                           pop      ds
00A499:  EB 34                        jmp      0xa4cf
00A49B:  65 F6 06 BB 0D 01            test     byte ptr gs:[0xdbb], 1
00A4A1:  75 0E                        jne      0xa4b1
00A4A3:  1E                           push     ds
00A4A4:  56                           push     si
00A4A5:  65 C5 36 29 52               lds      si, ptr gs:[0x5229]
00A4AA:  9A B6 0E 99 02               lcall    0x299, 0xeb6
00A4AF:  5E                           pop      si
00A4B0:  1F                           pop      ds
00A4B1:  65 80 3E BA 0D 00            cmp      byte ptr gs:[0xdba], 0
00A4B7:  75 19                        jne      0xa4d2
00A4B9:  0A C9                        or       cl, cl
00A4BB:  74 12                        je       0xa4cf
00A4BD:  65 F6 06 BD 0D 01            test     byte ptr gs:[0xdbd], 1
00A4C3:  75 07                        jne      0xa4cc
00A4C5:  80 F9 82                     cmp      cl, 0x82
00A4C8:  76 02                        jbe      0xa4cc
00A4CA:  B1 82                        mov      cl, 0x82
00A4CC:  E8 1E 00                     call     0xa4ed
00A4CF:  1F                           pop      ds
00A4D0:  EB 12                        jmp      0xa4e4
00A4D2:  33 FF                        xor      di, di
00A4D4:  65 8B 36 94 0D               mov      si, word ptr gs:[0xd94]
00A4D9:  0F A0                        push     fs
00A4DB:  E8 47 06                     call     0xab25
00A4DE:  0F A1                        pop      fs
00A4E0:  1F                           pop      ds
00A4E1:  B8 00 00                     mov      ax, 0
00A4E4:  5F                           pop      di
00A4E5:  07                           pop      es
00A4E6:  5E                           pop      si
00A4E7:  1F                           pop      ds
00A4E8:  5A                           pop      dx
00A4E9:  59                           pop      cx
00A4EA:  5B                           pop      bx
00A4EB:  5D                           pop      bp
00A4EC:  CB                           retf    
