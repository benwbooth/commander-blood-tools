; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00660c
; seg_off: 04da:126c
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a6_text
; label_comment: 0xA6 TEXT handler: resolves b1b2 in the far runtime record table; parses signed b3 plus b4/b5 skip, loop, condition, mode, and active controls; gates on presentation state; mutates accepted tokens; then assembles subtitle dictionary words or publishes the raw menu list before scanning to the zero terminator
; incoming: vm_opcode_handlers:opcode_0xa6
; byte_count: 411
; boundary: cfg_blocks_31_terminals_4
; terminal: jmp 0x66db:1, jmp 0x66ef:1, jmp 0x67a0:1, ret:1
; direct_callees: 0x006339, 0x00647b, 0x0067a7
; indirect_calls: 0
; routine_bytes_sha256: 5262aa18e4cddc870e1bc83ad81647861935024a0f9fb958a136d1128eab18c6

00660C:  57                           push     di
00660D:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006612:  AD                           lodsw    ax, word ptr [si]
006613:  03 F8                        add      di, ax
006615:  65 89 36 7C 67               mov      word ptr gs:[0x677c], si
00661A:  46                           inc      si
00661B:  AD                           lodsw    ax, word ptr [si]
00661C:  8B C8                        mov      cx, ax
00661E:  F6 C1 08                     test     cl, 8
006621:  74 0D                        je       0x6630
006623:  8A C5                        mov      al, ch
006625:  C0 E8 04                     shr      al, 4
006628:  24 07                        and      al, 7
00662A:  FE C0                        inc      al
00662C:  65 A2 AB 67                  mov      byte ptr gs:[0x67ab], al
006630:  F6 C1 10                     test     cl, 0x10
006633:  74 12                        je       0x6647
006635:  65 C6 06 B1 67 01            mov      byte ptr gs:[0x67b1], 1
00663B:  65 C7 06 64 67 00 00         mov      word ptr gs:[0x6764], 0
006642:  AD                           lodsw    ax, word ptr [si]
006643:  65 A3 78 67                  mov      word ptr gs:[0x6778], ax
006647:  0B C9                        or       cx, cx
006649:  0F 89 53 01                  jns      0x67a0
00664D:  65 A0 64 5E                  mov      al, byte ptr gs:[0x5e64]
006651:  65 0A 06 B0 67               or       al, byte ptr gs:[0x67b0]
006656:  0F 85 46 01                  jne      0x67a0
00665A:  26 F7 45 02 00 80            test     word ptr es:[di + 2], 0x8000
006660:  0F 85 3C 01                  jne      0x67a0
006664:  B8 13 00                     mov      ax, 0x13
006667:  C1 E0 04                     shl      ax, 4
00666A:  40                           inc      ax
00666B:  8B D8                        mov      bx, ax
00666D:  65 8A 87 60 6D               mov      al, byte ptr gs:[bx + 0x6d60]
006672:  98                           cwde    
006673:  8B D8                        mov      bx, ax
006675:  26 8B 11                     mov      dx, word ptr es:[bx + di]
006678:  81 FA C4 00                  cmp      dx, 0xc4
00667C:  0F 85 20 01                  jne      0x67a0
006680:  E8 B6 FC                     call     0x6339
006683:  0F 83 19 01                  jae      0x67a0
006687:  56                           push     si
006688:  65 8B 36 7C 67               mov      si, word ptr gs:[0x677c]
00668D:  AC                           lodsb    al, byte ptr [si]
00668E:  98                           cwde    
00668F:  65 A3 AB 1F                  mov      word ptr gs:[0x1fab], ax
006693:  F6 C1 01                     test     cl, 1
006696:  75 04                        jne      0x669c
006698:  80 64 01 7F                  and      byte ptr [si + 1], 0x7f
00669C:  5E                           pop      si
00669D:  F6 C1 04                     test     cl, 4
0066A0:  74 03                        je       0x66a5
0066A2:  83 C6 02                     add      si, 2
0066A5:  65 F6 06 B9 67 01            test     byte ptr gs:[0x67b9], 1
0066AB:  0F 84 AF 00                  je       0x675e
0066AF:  65 C6 06 FB 0C 01            mov      byte ptr gs:[0xcfb], 1
0066B5:  65 C6 06 FA 0C 00            mov      byte ptr gs:[0xcfa], 0
0066BB:  65 C6 06 B0 67 00            mov      byte ptr gs:[0x67b0], 0
0066C1:  65 C6 06 B9 67 00            mov      byte ptr gs:[0x67b9], 0
0066C7:  26 81 4D 02 00 80            or       word ptr es:[di + 2], 0x8000
0066CD:  8C E8                        mov      ax, gs
0066CF:  8E C0                        mov      es, ax
0066D1:  BF 18 0E                     mov      di, 0xe18
0066D4:  65 8B 1E 2A 67               mov      bx, word ptr gs:[0x672a]
0066D9:  32 D2                        xor      dl, dl
0066DB:  8B 04                        mov      ax, word ptr [si]
0066DD:  0B C0                        or       ax, ax
0066DF:  74 56                        je       0x6737
0066E1:  83 F8 FF                     cmp      ax, -1
0066E4:  74 51                        je       0x6737
0066E6:  83 C6 02                     add      si, 2
0066E9:  56                           push     si
0066EA:  1E                           push     ds
0066EB:  8B F0                        mov      si, ax
0066ED:  8E DB                        mov      ds, bx
0066EF:  AC                           lodsb    al, byte ptr [si]
0066F0:  0A C0                        or       al, al
0066F2:  74 05                        je       0x66f9
0066F4:  AA                           stosb    byte ptr es:[di], al
0066F5:  FE C2                        inc      dl
0066F7:  EB F6                        jmp      0x66ef
0066F9:  1F                           pop      ds
0066FA:  5E                           pop      si
0066FB:  06                           push     es
0066FC:  57                           push     di
0066FD:  8E C3                        mov      es, bx
0066FF:  8B 3C                        mov      di, word ptr [si]
006701:  E8 A3 00                     call     0x67a7
006704:  26 8A 25                     mov      ah, byte ptr es:[di]
006707:  5F                           pop      di
006708:  07                           pop      es
006709:  80 FC 2C                     cmp      ah, 0x2c
00670C:  74 CD                        je       0x66db
00670E:  80 FC 2E                     cmp      ah, 0x2e
006711:  74 C8                        je       0x66db
006713:  80 FC 3F                     cmp      ah, 0x3f
006716:  74 C3                        je       0x66db
006718:  80 FC 21                     cmp      ah, 0x21
00671B:  74 BE                        je       0x66db
00671D:  80 FC 3A                     cmp      ah, 0x3a
006720:  74 B9                        je       0x66db
006722:  B4 20                        mov      ah, 0x20
006724:  26 88 25                     mov      byte ptr es:[di], ah
006727:  47                           inc      di
006728:  FE C2                        inc      dl
00672A:  02 C2                        add      al, dl
00672C:  3C 23                        cmp      al, 0x23
00672E:  72 AB                        jb       0x66db
006730:  32 D2                        xor      dl, dl
006732:  B0 0D                        mov      al, 0xd
006734:  AA                           stosb    byte ptr es:[di], al
006735:  EB A4                        jmp      0x66db
006737:  B0 0D                        mov      al, 0xd
006739:  AA                           stosb    byte ptr es:[di], al
00673A:  32 C0                        xor      al, al
00673C:  AA                           stosb    byte ptr es:[di], al
00673D:  65 C6 06 64 5E 01            mov      byte ptr gs:[0x5e64], 1
006743:  65 C7 06 58 5E 00 00         mov      word ptr gs:[0x5e58], 0
00674A:  65 80 06 B4 67 02            add      byte ptr gs:[0x67b4], 2
006750:  65 C6 06 BC 67 00            mov      byte ptr gs:[0x67bc], 0
006756:  65 80 0E AA 67 01            or       byte ptr gs:[0x67aa], 1
00675C:  EB 42                        jmp      0x67a0
00675E:  E8 1A FD                     call     0x647b
006761:  26 81 4D 02 00 80            or       word ptr es:[di + 2], 0x8000
006767:  65 C6 06 64 5E 00            mov      byte ptr gs:[0x5e64], 0
00676D:  65 C6 06 F9 0C 01            mov      byte ptr gs:[0xcf9], 1
006773:  65 80 0E AA 67 01            or       byte ptr gs:[0x67aa], 1
006779:  65 80 06 B4 67 02            add      byte ptr gs:[0x67b4], 2
00677F:  65 C6 06 B0 67 01            mov      byte ptr gs:[0x67b0], 1
006785:  65 C6 06 BC 67 00            mov      byte ptr gs:[0x67bc], 0
00678B:  65 C6 06 B3 1F 01            mov      byte ptr gs:[0x1fb3], 1
006791:  65 89 36 D3 27               mov      word ptr gs:[0x27d3], si
006796:  65 89 36 4A 67               mov      word ptr gs:[0x674a], si
00679B:  65 8C 1E 4C 67               mov      word ptr gs:[0x674c], ds
0067A0:  AD                           lodsw    ax, word ptr [si]
0067A1:  0B C0                        or       ax, ax
0067A3:  75 FB                        jne      0x67a0
0067A5:  5F                           pop      di
0067A6:  C3                           ret     
