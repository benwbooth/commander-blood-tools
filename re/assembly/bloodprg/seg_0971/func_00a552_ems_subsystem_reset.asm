; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a552
; seg_off: 0971:0842
; group: seg_0971
; provenance: recursive_graph
; label: ems_subsystem_reset
; label_comment: reset the EMS-banked resource subsystem (2 calls): gs:[0xaa0]=0; gs:[0xdba]=0; [0xd9c]=0xffff. Clears the banked-list active/state pointers to their empty values
; byte_count: 208
; boundary: cfg_blocks_24_terminals_5
; terminal: jmp 0xa3d0:1, jmp 0xa578:1, jmp 0xa615:1, ret:2
; direct_callees: 0x00a634, 0x00a82c
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a552_ems_subsystem_reset.cpp
; routine_bytes_sha256: 3783f26b33e432594b2256290f2c67a208dce8cfaedb64e2415705212c3c6d4e

00A552:  65 C7 06 A0 0A 00 00         mov      word ptr gs:[0xaa0], 0
00A559:  65 C6 06 BA 0D 00            mov      byte ptr gs:[0xdba], 0
00A55F:  C7 06 9C 0D FF FF            mov      word ptr [0xd9c], 0xffff
00A565:  C7 06 9E 0D FF FF            mov      word ptr [0xd9e], 0xffff
00A56B:  03 C6                        add      ax, si
00A56D:  72 07                        jb       0xa576
00A56F:  65 3B 06 33 52               cmp      ax, word ptr gs:[0x5233]
00A574:  76 02                        jbe      0xa578
00A576:  33 F6                        xor      si, si
00A578:  26 AD                        lodsw    ax, word ptr es:[si]
00A57A:  3D 73 64                     cmp      ax, 0x6473
00A57D:  75 13                        jne      0xa592
00A57F:  E8 B2 00                     call     0xa634
00A582:  74 05                        je       0xa589
00A584:  65 89 36 9C 0D               mov      word ptr gs:[0xd9c], si
00A589:  26 AD                        lodsw    ax, word ptr es:[si]
00A58B:  83 E8 04                     sub      ax, 4
00A58E:  03 F0                        add      si, ax
00A590:  26 AD                        lodsw    ax, word ptr es:[si]
00A592:  3D 70 6C                     cmp      ax, 0x6c70
00A595:  75 0E                        jne      0xa5a5
00A597:  26 AD                        lodsw    ax, word ptr es:[si]
00A599:  65 89 36 9E 0D               mov      word ptr gs:[0xd9e], si
00A59E:  83 E8 04                     sub      ax, 4
00A5A1:  03 F0                        add      si, ax
00A5A3:  EB D3                        jmp      0xa578
00A5A5:  3D 6D 6D                     cmp      ax, 0x6d6d
00A5A8:  75 12                        jne      0xa5bc
00A5AA:  26 8B 5C 04                  mov      bx, word ptr es:[si + 4]
00A5AE:  26 C4 34                     les      si, ptr es:[si]
00A5B1:  26 AD                        lodsw    ax, word ptr es:[si]
00A5B3:  3B C3                        cmp      ax, bx
00A5B5:  26 AD                        lodsw    ax, word ptr es:[si]
00A5B7:  74 03                        je       0xa5bc
00A5B9:  E9 14 FE                     jmp      0xa3d0
00A5BC:  1E                           push     ds
00A5BD:  06                           push     es
00A5BE:  8E C5                        mov      es, bp
00A5C0:  33 FF                        xor      di, di
00A5C2:  F6 C4 04                     test     ah, 4
00A5C5:  74 05                        je       0xa5cc
00A5C7:  65 8E 06 BE 0A               mov      es, word ptr gs:[0xabe]
00A5CC:  65 8C 06 96 0D               mov      word ptr gs:[0xd96], es
00A5D1:  65 89 3E 94 0D               mov      word ptr gs:[0xd94], di
00A5D6:  65 A3 A4 0D                  mov      word ptr gs:[0xda4], ax
00A5DA:  1F                           pop      ds
00A5DB:  8B D8                        mov      bx, ax
00A5DD:  AB                           stosw    word ptr es:[di], ax
00A5DE:  AD                           lodsw    ax, word ptr [si]
00A5DF:  65 A3 A6 0D                  mov      word ptr gs:[0xda6], ax
00A5E3:  AB                           stosw    word ptr es:[di], ax
00A5E4:  8B C8                        mov      cx, ax
00A5E6:  0A C9                        or       cl, cl
00A5E8:  74 26                        je       0xa610
00A5EA:  F7 C3 00 02                  test     bx, 0x200
00A5EE:  74 22                        je       0xa612
00A5F0:  65 F6 06 BB 0D 01            test     byte ptr gs:[0xdbb], 1
00A5F6:  75 15                        jne      0xa60d
00A5F8:  65 F6 06 B9 0D 01            test     byte ptr gs:[0xdb9], 1
00A5FE:  75 0D                        jne      0xa60d
00A600:  80 FC FF                     cmp      ah, 0xff
00A603:  75 08                        jne      0xa60d
00A605:  65 C6 06 BA 0D 01            mov      byte ptr gs:[0xdba], 1
00A60B:  EB 08                        jmp      0xa615
00A60D:  E8 1C 02                     call     0xa82c
00A610:  1F                           pop      ds
00A611:  C3                           ret     
00A612:  83 EE 04                     sub      si, 4
00A615:  8C D8                        mov      ax, ds
00A617:  1F                           pop      ds
00A618:  65 89 36 94 0D               mov      word ptr gs:[0xd94], si
00A61D:  65 A3 96 0D                  mov      word ptr gs:[0xd96], ax
00A621:  C3                           ret     
