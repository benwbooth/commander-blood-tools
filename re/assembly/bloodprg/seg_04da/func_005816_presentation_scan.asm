; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005816
; seg_off: 04da:0476
; group: seg_04da
; provenance: recursive_graph
; label: presentation_scan
; label_comment: post-VM record scan: walks the 0x672c directory; ACTIVE records (+2 bit0) by kind: kind 2 = character display maintenance (primary C4 0x675E, wildcard related 0x674E, field ptr follow); kind 1 = PRESENTATION START when the record's C4 field (+0x13 field) is armed: sets 0x67AC=1 active + 0x67B7 start-lock + 0x2793|=4 busy, clears dialogue state (0x6782/0x6784/0x6776/0x67F8/0x67BA..0x67BC/0x679A), record +3 |= 0x80. Kinds 0x10/0x200 -> 0x5A51 shared path. Ported effect: VmMachine::start_actor_presentation + the engine dialogue lifecycle || ALSO RECORDED as `vm_post_exec_record_update`: post-VM scan of DEB/object table via DS:0x672c and runtime state via DS:0x6724; updates display/control flags || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; byte_count: 606
; boundary: cfg_blocks_39_terminals_4
; terminal: jmp 0x59f9:1, jmp 0x5a45:1, jmp 0x5a64:1, ret:1
; direct_callees: 0x0056fe, 0x005b38, 0x006023, 0x007409
; indirect_calls: 4
; routine_bytes_sha256: d7a3c80e01ade4bb58a57cef8f1c2cc75889e9627b2b15c6c2d1ad304752b0f7

005816:  57                           push     di
005817:  65 C6 06 B6 67 00            mov      byte ptr gs:[0x67b6], 0
00581D:  65 C5 36 24 67               lds      si, ptr gs:[0x6724]
005822:  65 C4 3E 2C 67               les      di, ptr gs:[0x672c]
005827:  66 33 C0                     xor      eax, eax
00582A:  56                           push     si
00582B:  66 33 F6                     xor      esi, esi
00582E:  5E                           pop      si
00582F:  26 8B 75 10                  mov      si, word ptr es:[di + 0x10]
005833:  F6 44 02 01                  test     byte ptr [si + 2], 1
005837:  0F 84 29 02                  je       0x5a64
00583B:  8B 1C                        mov      bx, word ptr [si]
00583D:  B8 13 00                     mov      ax, 0x13
005840:  E8 E0 07                     call     0x6023
005843:  03 C6                        add      ax, si
005845:  8B E8                        mov      bp, ax
005847:  83 FB 02                     cmp      bx, 2
00584A:  75 72                        jne      0x58be
00584C:  65 F6 06 AC 67 01            test     byte ptr gs:[0x67ac], 1
005852:  74 54                        je       0x58a8
005854:  65 F6 06 B2 1F 01            test     byte ptr gs:[0x1fb2], 1
00585A:  75 4C                        jne      0x58a8
00585C:  65 F6 06 D7 27 01            test     byte ptr gs:[0x27d7], 1
005862:  75 44                        jne      0x58a8
005864:  65 F6 06 B7 67 01            test     byte ptr gs:[0x67b7], 1
00586A:  75 3C                        jne      0x58a8
00586C:  65 8B 1E 5E 67               mov      bx, word ptr gs:[0x675e]
005871:  8B 07                        mov      ax, word ptr [bx]
005873:  3D C4 00                     cmp      ax, 0xc4
005876:  75 30                        jne      0x58a8
005878:  3E 8B 46 00                  mov      ax, word ptr ds:[bp]
00587C:  3D C4 00                     cmp      ax, 0xc4
00587F:  75 27                        jne      0x58a8
005881:  3E 8B 46 02                  mov      ax, word ptr ds:[bp + 2]
005885:  65 3B 06 4E 67               cmp      ax, word ptr gs:[0x674e]
00588A:  75 1C                        jne      0x58a8
00588C:  F7 44 02 00 80               test     word ptr [si + 2], 0x8000
005891:  75 15                        jne      0x58a8
005893:  8B 1C                        mov      bx, word ptr [si]
005895:  B8 02 00                     mov      ax, 2
005898:  E8 88 07                     call     0x6023
00589B:  66 98                        cwde    
00589D:  67 8B 1C 30                  mov      bx, word ptr [eax + esi]
0058A1:  0B DB                        or       bx, bx
0058A3:  74 03                        je       0x58a8
0058A5:  E8 56 FE                     call     0x56fe
0058A8:  3E 8B 46 04                  mov      ax, word ptr ds:[bp + 4]
0058AC:  0B C0                        or       ax, ax
0058AE:  78 0B                        js       0x58bb
0058B0:  3E 8B 46 00                  mov      ax, word ptr ds:[bp]
0058B4:  0B C0                        or       ax, ax
0058B6:  74 03                        je       0x58bb
0058B8:  E8 7D 02                     call     0x5b38
0058BB:  E9 A6 01                     jmp      0x5a64
0058BE:  83 FB 10                     cmp      bx, 0x10
0058C1:  0F 84 8C 01                  je       0x5a51
0058C5:  81 FB 00 02                  cmp      bx, 0x200
0058C9:  0F 84 84 01                  je       0x5a51
0058CD:  83 FB 01                     cmp      bx, 1
0058D0:  0F 85 90 01                  jne      0x5a64
0058D4:  3E 8B 46 00                  mov      ax, word ptr ds:[bp]
0058D8:  3D C4 00                     cmp      ax, 0xc4
0058DB:  0F 85 BB 00                  jne      0x599a
0058DF:  3E 8B 5E 02                  mov      bx, word ptr ds:[bp + 2]
0058E3:  F6 47 02 20                  test     byte ptr [bx + 2], 0x20
0058E7:  65 0F 95 06 AF 67            setne    byte ptr gs:[0x67af]
0058ED:  65 F6 06 AC 67 01            test     byte ptr gs:[0x67ac], 1
0058F3:  0F 85 02 01                  jne      0x59f9
0058F7:  65 C6 06 55 5B 01            mov      byte ptr gs:[0x5b55], 1
0058FD:  65 C7 06 32 0A 01 00         mov      word ptr gs:[0xa32], 1
005904:  65 C6 06 AC 67 01            mov      byte ptr gs:[0x67ac], 1
00590A:  33 C0                        xor      ax, ax
00590C:  65 A3 82 67                  mov      word ptr gs:[0x6782], ax
005910:  65 A3 84 67                  mov      word ptr gs:[0x6784], ax
005914:  65 A3 76 67                  mov      word ptr gs:[0x6776], ax
005918:  65 A3 F8 67                  mov      word ptr gs:[0x67f8], ax
00591C:  65 A3 19 2A                  mov      word ptr gs:[0x2a19], ax
005920:  65 A2 BA 67                  mov      byte ptr gs:[0x67ba], al
005924:  65 A2 D7 27                  mov      byte ptr gs:[0x27d7], al
005928:  65 A2 BC 67                  mov      byte ptr gs:[0x67bc], al
00592C:  65 A2 BB 67                  mov      byte ptr gs:[0x67bb], al
005930:  65 A3 9A 67                  mov      word ptr gs:[0x679a], ax
005934:  65 C6 06 B7 67 01            mov      byte ptr gs:[0x67b7], 1
00593A:  65 80 0E 93 27 04            or       byte ptr gs:[0x2793], 4
005940:  80 4F 03 80                  or       byte ptr [bx + 3], 0x80
005944:  65 80 26 51 27 7F            and      byte ptr gs:[0x2751], 0x7f
00594A:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
005950:  0F 84 A5 00                  je       0x59f9
005954:  57                           push     di
005955:  06                           push     es
005956:  8C D8                        mov      ax, ds
005958:  8E C0                        mov      es, ax
00595A:  8B FB                        mov      di, bx
00595C:  83 C7 04                     add      di, 4
00595F:  0E                           push     cs
005960:  E8 A6 1A                     call     0x7409
005963:  65 F6 06 E8 27 01            test     byte ptr gs:[0x27e8], 1
005969:  07                           pop      es
00596A:  5F                           pop      di
00596B:  0F 84 8A 00                  je       0x59f9
00596F:  65 C6 06 E9 27 01            mov      byte ptr gs:[0x27e9], 1
005975:  06                           push     es
005976:  57                           push     di
005977:  65 C4 3E 80 0A               les      di, ptr gs:[0xa80]
00597C:  B8 07 80                     mov      ax, 0x8007
00597F:  9A 37 10 99 02               lcall    0x299, 0x1037
005984:  55                           push     bp
005985:  B8 02 00                     mov      ax, 2
005988:  BB 10 00                     mov      bx, 0x10
00598B:  B9 4A 00                     mov      cx, 0x4a
00598E:  33 ED                        xor      bp, bp
005990:  9A BE 11 99 02               lcall    0x299, 0x11be
005995:  5D                           pop      bp
005996:  5F                           pop      di
005997:  07                           pop      es
005998:  EB 5F                        jmp      0x59f9
00599A:  65 F6 06 AC 67 01            test     byte ptr gs:[0x67ac], 1
0059A0:  74 57                        je       0x59f9
0059A2:  65 C7 06 32 0A 01 00         mov      word ptr gs:[0xa32], 1
0059A9:  33 C0                        xor      ax, ax
0059AB:  65 A3 82 67                  mov      word ptr gs:[0x6782], ax
0059AF:  65 A3 84 67                  mov      word ptr gs:[0x6784], ax
0059B3:  65 A2 B1 67                  mov      byte ptr gs:[0x67b1], al
0059B7:  65 A2 AC 67                  mov      byte ptr gs:[0x67ac], al
0059BB:  65 A3 62 67                  mov      word ptr gs:[0x6762], ax
0059BF:  65 83 26 93 27 FB            and      word ptr gs:[0x2793], 0xfffb
0059C5:  65 80 26 AA 67 FC            and      byte ptr gs:[0x67aa], 0xfc
0059CB:  65 A3 F8 67                  mov      word ptr gs:[0x67f8], ax
0059CF:  65 A2 B7 67                  mov      byte ptr gs:[0x67b7], al
0059D3:  65 C6 06 E8 27 00            mov      byte ptr gs:[0x27e8], 0
0059D9:  B8 04 00                     mov      ax, 4
0059DC:  9A 41 12 99 02               lcall    0x299, 0x1241
0059E1:  B8 02 00                     mov      ax, 2
0059E4:  9A 41 12 99 02               lcall    0x299, 0x1241
0059E9:  06                           push     es
0059EA:  57                           push     di
0059EB:  65 C4 3E 46 67               les      di, ptr gs:[0x6746]
0059F0:  B9 08 00                     mov      cx, 8
0059F3:  33 C0                        xor      ax, ax
0059F5:  F3 AB                        rep stosw word ptr es:[di], ax
0059F7:  5F                           pop      di
0059F8:  07                           pop      es
0059F9:  65 8B 16 6A 67               mov      dx, word ptr gs:[0x676a]
0059FE:  0B D2                        or       dx, dx
005A00:  74 4F                        je       0x5a51
005A02:  65 8B 0E 68 67               mov      cx, word ptr gs:[0x6768]
005A07:  0B C9                        or       cx, cx
005A09:  74 46                        je       0x5a51
005A0B:  81 F9 C1 00                  cmp      cx, 0xc1
005A0F:  74 06                        je       0x5a17
005A11:  81 F9 C6 00                  cmp      cx, 0xc6
005A15:  75 1C                        jne      0x5a33
005A17:  BB 10 00                     mov      bx, 0x10
005A1A:  B8 13 00                     mov      ax, 0x13
005A1D:  E8 03 06                     call     0x6023
005A20:  65 8B 1E 52 67               mov      bx, word ptr gs:[0x6752]
005A25:  03 D8                        add      bx, ax
005A27:  89 0F                        mov      word ptr [bx], cx
005A29:  89 57 02                     mov      word ptr [bx + 2], dx
005A2C:  33 C0                        xor      ax, ax
005A2E:  89 47 04                     mov      word ptr [bx + 4], ax
005A31:  EB 12                        jmp      0x5a45
005A33:  3E 89 4E 00                  mov      word ptr ds:[bp], cx
005A37:  3E 89 56 02                  mov      word ptr ds:[bp + 2], dx
005A3B:  65 A1 6C 67                  mov      ax, word ptr gs:[0x676c]
005A3F:  3E 89 46 04                  mov      word ptr ds:[bp + 4], ax
005A43:  33 C0                        xor      ax, ax
005A45:  65 A3 68 67                  mov      word ptr gs:[0x6768], ax
005A49:  65 A3 6A 67                  mov      word ptr gs:[0x676a], ax
005A4D:  65 A3 6C 67                  mov      word ptr gs:[0x676c], ax
005A51:  3E 8B 46 04                  mov      ax, word ptr ds:[bp + 4]
005A55:  0B C0                        or       ax, ax
005A57:  78 0B                        js       0x5a64
005A59:  3E 8B 46 00                  mov      ax, word ptr ds:[bp]
005A5D:  0B C0                        or       ax, ax
005A5F:  74 03                        je       0x5a64
005A61:  E8 D4 00                     call     0x5b38
005A64:  83 C7 14                     add      di, 0x14
005A67:  26 8B 45 12                  mov      ax, word ptr es:[di + 0x12]
005A6B:  83 F8 01                     cmp      ax, 1
005A6E:  0F 84 B8 FD                  je       0x582a
005A72:  5F                           pop      di
005A73:  C3                           ret     
