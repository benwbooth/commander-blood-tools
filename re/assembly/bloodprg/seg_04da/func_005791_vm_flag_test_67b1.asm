; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005791
; seg_off: 04da:03f1
; group: seg_04da
; provenance: recursive_graph
; label: vm_flag_test_67b1
; label_comment: VM flag test: or ax,ax; if zero skip to 0x5811, else test byte gs:[0x67b1],2. Branches on the 0x67b1 VM state bits
; byte_count: 133
; boundary: cfg_blocks_11_terminals_2
; terminal: jmp 0x580a:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: eec63db9842c209da711e71fbd088b983c51906d3e16cd40930294e85f32150d

005791:  06                           push     es
005792:  57                           push     di
005793:  1E                           push     ds
005794:  56                           push     si
005795:  65 A1 62 67                  mov      ax, word ptr gs:[0x6762]
005799:  0B C0                        or       ax, ax
00579B:  74 74                        je       0x5811
00579D:  65 F6 06 B1 67 02            test     byte ptr gs:[0x67b1], 2
0057A3:  74 04                        je       0x57a9
0057A5:  65 A3 64 67                  mov      word ptr gs:[0x6764], ax
0057A9:  65 C7 06 F8 67 00 00         mov      word ptr gs:[0x67f8], 0
0057B0:  65 C4 3E 46 67               les      di, ptr gs:[0x6746]
0057B5:  65 8B 1E 44 67               mov      bx, word ptr gs:[0x6744]
0057BA:  03 FB                        add      di, bx
0057BC:  AB                           stosw    word ptr es:[di], ax
0057BD:  83 C3 02                     add      bx, 2
0057C0:  83 E3 0F                     and      bx, 0xf
0057C3:  65 89 1E 44 67               mov      word ptr gs:[0x6744], bx
0057C8:  65 8E 1E 22 67               mov      ds, word ptr gs:[0x6722]
0057CD:  65 8B 36 76 67               mov      si, word ptr gs:[0x6776]
0057D2:  0B F6                        or       si, si
0057D4:  74 34                        je       0x580a
0057D6:  3B 04                        cmp      ax, word ptr [si]
0057D8:  74 09                        je       0x57e3
0057DA:  8B 74 02                     mov      si, word ptr [si + 2]
0057DD:  0B F6                        or       si, si
0057DF:  75 F5                        jne      0x57d6
0057E1:  EB 27                        jmp      0x580a
0057E3:  83 C6 04                     add      si, 4
0057E6:  8A 1C                        mov      bl, byte ptr [si]
0057E8:  80 FB A3                     cmp      bl, 0xa3
0057EB:  75 1D                        jne      0x580a
0057ED:  65 8B 1E 82 67               mov      bx, word ptr gs:[0x6782]
0057F2:  65 89 1E 84 67               mov      word ptr gs:[0x6784], bx
0057F7:  65 8B 1E 72 67               mov      bx, word ptr gs:[0x6772]
0057FC:  65 89 1E 74 67               mov      word ptr gs:[0x6774], bx
005801:  65 A3 82 67                  mov      word ptr gs:[0x6782], ax
005805:  65 89 36 72 67               mov      word ptr gs:[0x6772], si
00580A:  65 C7 06 62 67 00 00         mov      word ptr gs:[0x6762], 0
005811:  5E                           pop      si
005812:  1F                           pop      ds
005813:  5F                           pop      di
005814:  07                           pop      es
005815:  C3                           ret     
