; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0082e8
; seg_off: 071e:0b08
; group: seg_071e
; provenance: recursive_graph
; label: nav_state_gate
; label_comment: per-frame nav/state gate: test [0x27da]&1 and [0x278a]&1 -> branch. Guards the nav/camera update flow based on the transition flags
; byte_count: 320
; boundary: cfg_blocks_38_terminals_9
; terminal: jmp 0x8347:1, jmp 0x8381:1, jmp 0x8391:1, jmp 0x83a2:1, jmp 0x83cc:1, jmp 0x83ed:1, jmp 0x840e:1, jmp 0x8420:1, ret:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_071e/func_0082e8_nav_state_gate.cpp
; routine_bytes_sha256: a7e9873e82a6dc93b9209d39a05cde5b2b2d3066ad7e0f2689f4da517a9657d9

0082E8:  50                           push     ax
0082E9:  57                           push     di
0082EA:  56                           push     si
0082EB:  0F A0                        push     fs
0082ED:  1E                           push     ds
0082EE:  55                           push     bp
0082EF:  F6 06 DA 27 01               test     byte ptr [0x27da], 1
0082F4:  0F 85 28 01                  jne      0x8420
0082F8:  F6 06 8A 27 01               test     byte ptr [0x278a], 1
0082FD:  0F 85 1F 01                  jne      0x8420
008301:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
008306:  0F 85 16 01                  jne      0x8420
00830A:  BE F2 65                     mov      si, 0x65f2
00830D:  8A 04                        mov      al, byte ptr [si]
00830F:  A8 01                        test     al, 1
008311:  0F 84 0B 01                  je       0x8420
008315:  83 C6 08                     add      si, 8
008318:  AD                           lodsw    ax, word ptr [si]
008319:  3B 06 2A 0A                  cmp      ax, word ptr [0xa2a]
00831D:  77 20                        ja       0x833f
00831F:  03 44 02                     add      ax, word ptr [si + 2]
008322:  3B 06 2A 0A                  cmp      ax, word ptr [0xa2a]
008326:  72 17                        jb       0x833f
008328:  AD                           lodsw    ax, word ptr [si]
008329:  3B 06 2C 0A                  cmp      ax, word ptr [0xa2c]
00832D:  77 10                        ja       0x833f
00832F:  03 44 02                     add      ax, word ptr [si + 2]
008332:  3B 06 2C 0A                  cmp      ax, word ptr [0xa2c]
008336:  72 07                        jb       0x833f
008338:  80 0E E2 27 01               or       byte ptr [0x27e2], 1
00833D:  EB 08                        jmp      0x8347
00833F:  C6 06 E2 27 00               mov      byte ptr [0x27e2], 0
008344:  E9 D9 00                     jmp      0x8420
008347:  F6 06 E2 27 03               test     byte ptr [0x27e2], 3
00834C:  0F 84 D0 00                  je       0x8420
008350:  F6 06 E2 27 02               test     byte ptr [0x27e2], 2
008355:  0F 85 C7 00                  jne      0x8420
008359:  BF 18 0E                     mov      di, 0xe18
00835C:  8B 36 52 67                  mov      si, word ptr [0x6752]
008360:  66 8E 26 26 67               mov      fs, word ptr [0x6726]
008365:  64 8B 6C 16                  mov      bp, word ptr fs:[si + 0x16]
008369:  BE 2E 01                     mov      si, 0x12e
00836C:  64 83 7E 00 10               cmp      word ptr fs:[bp], 0x10
008371:  75 03                        jne      0x8376
008373:  BE 37 01                     mov      si, 0x137
008376:  64 F7 46 00 00 01            test     word ptr fs:[bp], 0x100
00837C:  74 03                        je       0x8381
00837E:  BE 3E 01                     mov      si, 0x13e
008381:  AC                           lodsb    al, byte ptr [si]
008382:  0A C0                        or       al, al
008384:  74 03                        je       0x8389
008386:  AA                           stosb    byte ptr es:[di], al
008387:  EB F8                        jmp      0x8381
008389:  8B F5                        mov      si, bp
00838B:  83 C6 04                     add      si, 4
00838E:  0F A0                        push     fs
008390:  1F                           pop      ds
008391:  AC                           lodsb    al, byte ptr [si]
008392:  0A C0                        or       al, al
008394:  74 03                        je       0x8399
008396:  AA                           stosb    byte ptr es:[di], al
008397:  EB F8                        jmp      0x8391
008399:  0F A8                        push     gs
00839B:  1F                           pop      ds
00839C:  B0 0D                        mov      al, 0xd
00839E:  AA                           stosb    byte ptr es:[di], al
00839F:  BE 4B 01                     mov      si, 0x14b
0083A2:  AC                           lodsb    al, byte ptr [si]
0083A3:  0A C0                        or       al, al
0083A5:  74 03                        je       0x83aa
0083A7:  AA                           stosb    byte ptr es:[di], al
0083A8:  EB F8                        jmp      0x83a2
0083AA:  B0 0D                        mov      al, 0xd
0083AC:  AA                           stosb    byte ptr es:[di], al
0083AD:  57                           push     di
0083AE:  8C E0                        mov      ax, fs
0083B0:  8E C0                        mov      es, ax
0083B2:  8E D8                        mov      ds, ax
0083B4:  8B FD                        mov      di, bp
0083B6:  BD 86 68                     mov      bp, 0x6886
0083B9:  9A AB 0E DA 04               lcall    0x4da, 0xeab
0083BE:  BD 86 68                     mov      bp, 0x6886
0083C1:  5F                           pop      di
0083C2:  0F A8                        push     gs
0083C4:  07                           pop      es
0083C5:  32 E4                        xor      ah, ah
0083C7:  65 8B 1E 58 67               mov      bx, word ptr gs:[0x6758]
0083CC:  8B 76 00                     mov      si, word ptr [bp]
0083CF:  83 FE FF                     cmp      si, -1
0083D2:  74 29                        je       0x83fd
0083D4:  83 3C 02                     cmp      word ptr [si], 2
0083D7:  75 1F                        jne      0x83f8
0083D9:  F6 44 02 01                  test     byte ptr [si + 2], 1
0083DD:  74 19                        je       0x83f8
0083DF:  83 7C 36 00                  cmp      word ptr [si + 0x36], 0
0083E3:  74 13                        je       0x83f8
0083E5:  39 5C 18                     cmp      word ptr [si + 0x18], bx
0083E8:  74 0E                        je       0x83f8
0083EA:  83 C6 04                     add      si, 4
0083ED:  AC                           lodsb    al, byte ptr [si]
0083EE:  0A C0                        or       al, al
0083F0:  74 03                        je       0x83f5
0083F2:  AA                           stosb    byte ptr es:[di], al
0083F3:  EB F8                        jmp      0x83ed
0083F5:  B0 0D                        mov      al, 0xd
0083F7:  AA                           stosb    byte ptr es:[di], al
0083F8:  83 C5 02                     add      bp, 2
0083FB:  EB CF                        jmp      0x83cc
0083FD:  EB 0F                        jmp      0x840e
; -- non-contiguous block: next 0x00840e --
00840E:  B0 0D                        mov      al, 0xd
008410:  AA                           stosb    byte ptr es:[di], al
008411:  32 C0                        xor      al, al
008413:  AA                           stosb    byte ptr es:[di], al
008414:  65 FE 06 E2 27               inc      byte ptr gs:[0x27e2]
008419:  65 C7 06 58 5E 00 00         mov      word ptr gs:[0x5e58], 0
008420:  5D                           pop      bp
008421:  1F                           pop      ds
008422:  0F A1                        pop      fs
008424:  5E                           pop      si
008425:  5F                           pop      di
008426:  58                           pop      ax
008427:  C3                           ret     
