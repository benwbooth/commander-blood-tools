; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0093f5
; seg_off: 071e:1c15
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: screen_mode_check
; label_comment: screen-mode gate: test [0x27e2]&2 and [0x5e64]&1 -> branch. Guards a screen/render mode-specific path
; incoming: call@0x0012bd->071e:1c15
; incoming: call@0x00be52->071e:1c15
; byte_count: 283
; boundary: cfg_blocks_27_terminals_6
; terminal: jmp 0x9457:2, jmp 0x94e2:1, jmp 0x94ee:1, jmp 0x950a:1, retf:1
; direct_callees: none
; indirect_calls: 3
; routine_bytes_sha256: 970702f7ce87cbad4c1f148d20926131674a76d5b3bb15b234927e5c94ba7639

0093F5:  66 50                        push     eax
0093F7:  1E                           push     ds
0093F8:  56                           push     si
0093F9:  55                           push     bp
0093FA:  F6 06 E2 27 02               test     byte ptr [0x27e2], 2
0093FF:  75 1A                        jne      0x941b
009401:  F6 06 64 5E 01               test     byte ptr [0x5e64], 1
009406:  75 13                        jne      0x941b
009408:  F6 06 BC 67 01               test     byte ptr [0x67bc], 1
00940D:  0F 84 F9 00                  je       0x950a
009411:  81 3E 9A 67 64 5E            cmp      word ptr [0x679a], 0x5e64
009417:  0F 85 EF 00                  jne      0x950a
00941B:  BE 18 0E                     mov      si, 0xe18
00941E:  8B 1E 58 5E                  mov      bx, word ptr [0x5e58]
009422:  0B DB                        or       bx, bx
009424:  75 16                        jne      0x943c
009426:  C7 06 31 0B 02 00            mov      word ptr [0xb31], 2
00942C:  C7 06 37 0B 01 00            mov      word ptr [0xb37], 1
009432:  89 36 58 5E                  mov      word ptr [0x5e58], si
009436:  C7 06 65 5E 02 00            mov      word ptr [0x5e65], 2
00943C:  B8 FF 00                     mov      ax, 0xff
00943F:  BD 6F 5E                     mov      bp, 0x5e6f
009442:  8B 1E 65 5E                  mov      bx, word ptr [0x5e65]
009446:  83 FB 02                     cmp      bx, 2
009449:  74 0C                        je       0x9457
00944B:  48                           dec      ax
00944C:  4B                           dec      bx
00944D:  74 08                        je       0x9457
00944F:  BD AF 5E                     mov      bp, 0x5eaf
009452:  C6 06 56 5B 01               mov      byte ptr [0x5b56], 1
009457:  8B 7E 00                     mov      di, word ptr [bp]
00945A:  0B FF                        or       di, di
00945C:  78 1D                        js       0x947b
00945E:  8B 56 06                     mov      dx, word ptr [bp + 6]
009461:  8B 5E 02                     mov      bx, word ptr [bp + 2]
009464:  8B 4E 04                     mov      cx, word ptr [bp + 4]
009467:  83 C5 08                     add      bp, 8
00946A:  4F                           dec      di
00946B:  74 07                        je       0x9474
00946D:  9A 2B 0A 99 02               lcall    0x299, 0xa2b
009472:  EB E3                        jmp      0x9457
009474:  9A 23 0B 99 02               lcall    0x299, 0xb23
009479:  EB DC                        jmp      0x9457
00947B:  C6 06 56 5B 00               mov      byte ptr [0x5b56], 0
009480:  A1 65 5E                     mov      ax, word ptr [0x5e65]
009483:  0B C0                        or       ax, ax
009485:  74 13                        je       0x949a
009487:  A1 37 0B                     mov      ax, word ptr [0xb37]
00948A:  0B C0                        or       ax, ax
00948C:  75 7C                        jne      0x950a
00948E:  C7 06 37 0B 01 00            mov      word ptr [0xb37], 1
009494:  FF 0E 65 5E                  dec      word ptr [0x5e65]
009498:  EB 70                        jmp      0x950a
00949A:  8B 1E 58 5E                  mov      bx, word ptr [0x5e58]
00949E:  8A 07                        mov      al, byte ptr [bx]
0094A0:  0A C0                        or       al, al
0094A2:  74 16                        je       0x94ba
0094A4:  A1 31 0B                     mov      ax, word ptr [0xb31]
0094A7:  0B C0                        or       ax, ax
0094A9:  75 37                        jne      0x94e2
0094AB:  A1 CA 0A                     mov      ax, word ptr [0xaca]
0094AE:  C1 E8 02                     shr      ax, 2
0094B1:  A3 31 0B                     mov      word ptr [0xb31], ax
0094B4:  FF 06 58 5E                  inc      word ptr [0x5e58]
0094B8:  EB 28                        jmp      0x94e2
0094BA:  F6 06 F3 24 04               test     byte ptr [0x24f3], 4
0094BF:  75 21                        jne      0x94e2
0094C1:  F6 06 BB 67 01               test     byte ptr [0x67bb], 1
0094C6:  75 1A                        jne      0x94e2
0094C8:  F6 06 BC 67 01               test     byte ptr [0x67bc], 1
0094CD:  75 13                        jne      0x94e2
0094CF:  C6 06 FB 0C 00               mov      byte ptr [0xcfb], 0
0094D4:  A1 CA 0A                     mov      ax, word ptr [0xaca]
0094D7:  C1 E0 02                     shl      ax, 2
0094DA:  A3 35 0B                     mov      word ptr [0xb35], ax
0094DD:  C6 06 BB 67 01               mov      byte ptr [0x67bb], 1
0094E2:  8C D8                        mov      ax, ds
0094E4:  8E C0                        mov      es, ax
0094E6:  8B 1E 5C 5E                  mov      bx, word ptr [0x5e5c]
0094EA:  8B 16 5E 5E                  mov      dx, word ptr [0x5e5e]
0094EE:  9A A0 06 99 02               lcall    0x299, 0x6a0
0094F3:  8B FE                        mov      di, si
0094F5:  B9 FF FF                     mov      cx, 0xffff
0094F8:  B0 0D                        mov      al, 0xd
0094FA:  F2 AE                        repne scasb al, byte ptr es:[di]
0094FC:  26 8A 05                     mov      al, byte ptr es:[di]
0094FF:  0A C0                        or       al, al
009501:  74 07                        je       0x950a
009503:  83 C2 08                     add      dx, 8
009506:  8B F7                        mov      si, di
009508:  EB E4                        jmp      0x94ee
00950A:  5D                           pop      bp
00950B:  5E                           pop      si
00950C:  1F                           pop      ds
00950D:  66 58                        pop      eax
00950F:  CB                           retf    
