; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0056fe
; seg_off: 04da:035e
; group: seg_04da
; provenance: recursive_graph
; label: vm_control_flow
; label_comment: select an object control value from selector-0x0f field, first code node, or GS:0x6782 override; write it back, scan linked code nodes, execute the matched block plus collector, then optionally execute the GS:0x6784 parent block; natural C: re/source/bloodprg/candidates/seg_04da/func_0056fe_vm_control_flow.c
; byte_count: 124
; boundary: cfg_blocks_10_terminals_1
; terminal: ret:1
; direct_callees: 0x0056a6, 0x00577a, 0x005afd, 0x006023
; indirect_calls: 0
; routine_bytes_sha256: 016affce6332a2dc45e2ede986dcb42bddb2b8b0f520713d728bacb063e3d304

0056FE:  1E                           push     ds
0056FF:  56                           push     si
005700:  06                           push     es
005701:  57                           push     di
005702:  53                           push     bx
005703:  55                           push     bp
005704:  8B FE                        mov      di, si
005706:  8C D8                        mov      ax, ds
005708:  8E C0                        mov      es, ax
00570A:  65 C5 36 20 67               lds      si, ptr gs:[0x6720]
00570F:  65 C6 06 B2 67 01            mov      byte ptr gs:[0x67b2], 1
005715:  8B F3                        mov      si, bx
005717:  46                           inc      si
005718:  65 89 36 76 67               mov      word ptr gs:[0x6776], si
00571D:  26 8B 1D                     mov      bx, word ptr es:[di]
005720:  B8 0F 00                     mov      ax, 0xf
005723:  E8 FD 08                     call     0x6023
005726:  03 C7                        add      ax, di
005728:  8B D8                        mov      bx, ax
00572A:  26 8B 07                     mov      ax, word ptr es:[bx]
00572D:  0B C0                        or       ax, ax
00572F:  75 02                        jne      0x5733
005731:  8B 04                        mov      ax, word ptr [si]
005733:  65 8B 16 82 67               mov      dx, word ptr gs:[0x6782]
005738:  0B D2                        or       dx, dx
00573A:  75 02                        jne      0x573e
00573C:  8B D0                        mov      dx, ax
00573E:  8B C2                        mov      ax, dx
005740:  26 89 07                     mov      word ptr es:[bx], ax
005743:  65 A3 82 67                  mov      word ptr gs:[0x6782], ax
005747:  E8 30 00                     call     0x577a
00574A:  0B C0                        or       ax, ax
00574C:  74 0C                        je       0x575a
00574E:  65 A3 72 67                  mov      word ptr gs:[0x6772], ax
005752:  8B F0                        mov      si, ax
005754:  E8 4F FF                     call     0x56a6
005757:  E8 A3 03                     call     0x5afd
00575A:  65 A1 84 67                  mov      ax, word ptr gs:[0x6784]
00575E:  0B C0                        or       ax, ax
005760:  74 11                        je       0x5773
005762:  65 8B 36 76 67               mov      si, word ptr gs:[0x6776]
005767:  E8 10 00                     call     0x577a
00576A:  0B C0                        or       ax, ax
00576C:  74 05                        je       0x5773
00576E:  8B F0                        mov      si, ax
005770:  E8 33 FF                     call     0x56a6
005773:  5D                           pop      bp
005774:  5B                           pop      bx
005775:  5F                           pop      di
005776:  07                           pop      es
005777:  5E                           pop      si
005778:  1F                           pop      ds
005779:  C3                           ret     
