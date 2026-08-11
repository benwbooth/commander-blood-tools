; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b7e3
; seg_off: 0b1b:0033
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: audio_process_ade
; label_comment: audio processing routine gated on [0xade]&1 (sound) + [0xadf]&1; part of the SND playback path (feeds the driver callback / mixer)
; incoming: call@0x001276->0b1b:0033
; byte_count: 234
; boundary: cfg_blocks_27_terminals_4
; terminal: jmp 0xb816:1, jmp 0xb822:1, jmp 0xb898:1, retf:1
; direct_callees: 0x00b8cd
; indirect_calls: 1
; routine_bytes_sha256: 64cde5b00846af7e05e9d3bc48fa6088a5d2381d09d16bb322768d0162eec490

00B7E3:  50                           push     ax
00B7E4:  53                           push     bx
00B7E5:  51                           push     cx
00B7E6:  52                           push     dx
00B7E7:  55                           push     bp
00B7E8:  57                           push     di
00B7E9:  56                           push     si
00B7EA:  1E                           push     ds
00B7EB:  06                           push     es
00B7EC:  F6 06 DE 0A 01               test     byte ptr [0xade], 1
00B7F1:  0F 84 CE 00                  je       0xb8c3
00B7F5:  F6 06 DF 0A 01               test     byte ptr [0xadf], 1
00B7FA:  0F 85 9A 00                  jne      0xb898
00B7FE:  F6 06 F9 0C 01               test     byte ptr [0xcf9], 1
00B803:  74 3F                        je       0xb844
00B805:  C6 06 F9 0C 00               mov      byte ptr [0xcf9], 0
00B80A:  C4 3E 28 67                  les      di, ptr [0x6728]
00B80E:  C5 36 4A 67                  lds      si, ptr [0x674a]
00B812:  33 DB                        xor      bx, bx
00B814:  33 D2                        xor      dx, dx
00B816:  AD                           lodsw    ax, word ptr [si]
00B817:  0B C0                        or       ax, ax
00B819:  74 17                        je       0xb832
00B81B:  83 F8 FF                     cmp      ax, -1
00B81E:  74 12                        je       0xb832
00B820:  8B F8                        mov      di, ax
00B822:  26 8A 05                     mov      al, byte ptr es:[di]
00B825:  47                           inc      di
00B826:  0A C0                        or       al, al
00B828:  74 05                        je       0xb82f
00B82A:  98                           cwde    
00B82B:  03 D8                        add      bx, ax
00B82D:  EB F3                        jmp      0xb822
00B82F:  42                           inc      dx
00B830:  EB E4                        jmp      0xb816
00B832:  03 DA                        add      bx, dx
00B834:  C1 EB 04                     shr      bx, 4
00B837:  65 89 1E 55 0C               mov      word ptr gs:[0xc55], bx
00B83C:  65 C6 06 FA 0C 01            mov      byte ptr gs:[0xcfa], 1
00B842:  EB 54                        jmp      0xb898
00B844:  F6 06 FA 0C 01               test     byte ptr [0xcfa], 1
00B849:  74 4D                        je       0xb898
00B84B:  83 3E 33 0B 00               cmp      word ptr [0xb33], 0
00B850:  75 46                        jne      0xb898
00B852:  8B 1E 55 0C                  mov      bx, word ptr [0xc55]
00B856:  80 E3 0F                     and      bl, 0xf
00B859:  A0 BD 0B                     mov      al, byte ptr [0xbbd]
00B85C:  02 C3                        add      al, bl
00B85E:  D0 EB                        shr      bl, 1
00B860:  3A 06 BE 0B                  cmp      al, byte ptr [0xbbe]
00B864:  77 F3                        ja       0xb859
00B866:  98                           cwde    
00B867:  A3 33 0B                     mov      word ptr [0xb33], ax
00B86A:  8B 1E 55 0C                  mov      bx, word ptr [0xc55]
00B86E:  8B C3                        mov      ax, bx
00B870:  83 EB 02                     sub      bx, 2
00B873:  8B D3                        mov      dx, bx
00B875:  83 E2 1F                     and      dx, 0x1f
00B878:  2B C2                        sub      ax, dx
00B87A:  79 02                        jns      0xb87e
00B87C:  F7 D8                        neg      ax
00B87E:  3B 06 53 0C                  cmp      ax, word ptr [0xc53]
00B882:  73 EC                        jae      0xb870
00B884:  FF 06 55 0C                  inc      word ptr [0xc55]
00B888:  3B 06 4D 0C                  cmp      ax, word ptr [0xc4d]
00B88C:  74 E2                        je       0xb870
00B88E:  A3 4D 0C                     mov      word ptr [0xc4d], ax
00B891:  0D 00 80                     or       ax, 0x8000
00B894:  0E                           push     cs
00B895:  E8 35 00                     call     0xb8cd
00B898:  F6 06 FB 0C 01               test     byte ptr [0xcfb], 1
00B89D:  74 24                        je       0xb8c3
00B89F:  80 3E 2F 0B 00               cmp      byte ptr [0xb2f], 0
00B8A4:  75 1D                        jne      0xb8c3
00B8A6:  C6 06 2F 0B 04               mov      byte ptr [0xb2f], 4
00B8AB:  B8 0A 00                     mov      ax, 0xa
00B8AE:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
00B8B3:  3B 06 4D 0C                  cmp      ax, word ptr [0xc4d]
00B8B7:  74 F2                        je       0xb8ab
00B8B9:  A3 4D 0C                     mov      word ptr [0xc4d], ax
00B8BC:  83 C0 07                     add      ax, 7
00B8BF:  0E                           push     cs
00B8C0:  E8 0A 00                     call     0xb8cd
00B8C3:  07                           pop      es
00B8C4:  1F                           pop      ds
00B8C5:  5E                           pop      si
00B8C6:  5F                           pop      di
00B8C7:  5D                           pop      bp
00B8C8:  5A                           pop      dx
00B8C9:  59                           pop      cx
00B8CA:  5B                           pop      bx
00B8CB:  58                           pop      ax
00B8CC:  CB                           retf    
