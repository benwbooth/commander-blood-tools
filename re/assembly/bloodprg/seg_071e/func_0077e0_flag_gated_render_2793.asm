; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0077e0
; seg_off: 071e:0000
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: flag_gated_render_2793
; label_comment: flag-gated render: ax=[0x2793]; test al,1; if clear skip to 0x78c9, else es=gs and render. Gates a render pass on the 0x2793 state bit
; incoming: call@0x001269->071e:0000
; byte_count: 240
; boundary: cfg_blocks_20_terminals_3
; terminal: jmp 0x7884:1, jmp 0x78c9:1, retf:1
; direct_callees: 0x0078d0, 0x00792d, 0x0079e5, 0x007d7b, 0x0082e8, 0x0085e2, 0x008a4e, 0x008bab, 0x008cce, 0x009510, 0x00954a, 0x00959d, 0x009656
; indirect_calls: 6
; cxx_source: re/borland/bloodprg/seg_071e/func_0077e0_flag_gated_render_2793.cpp
; routine_bytes_sha256: fd36735a458af5a55d7422bf38c310ba96093914eb821ddf91d58859960bd86d

0077E0:  66 50                        push     eax
0077E2:  53                           push     bx
0077E3:  51                           push     cx
0077E4:  52                           push     dx
0077E5:  55                           push     bp
0077E6:  A1 93 27                     mov      ax, word ptr [0x2793]
0077E9:  A8 01                        test     al, 1
0077EB:  0F 84 DA 00                  je       0x78c9
0077EF:  8C EB                        mov      bx, gs
0077F1:  8E C3                        mov      es, bx
0077F3:  F6 06 92 27 02               test     byte ptr [0x2792], 2
0077F8:  74 08                        je       0x7802
0077FA:  9A 00 00 71 09               lcall    0x971, 0
0077FF:  E9 C7 00                     jmp      0x78c9
007802:  F6 06 D9 27 01               test     byte ptr [0x27d9], 1
007807:  74 0F                        je       0x7818
007809:  C7 06 32 0A 01 00            mov      word ptr [0xa32], 1
00780F:  C7 06 36 0A 01 00            mov      word ptr [0xa36], 1
007815:  E8 85 1D                     call     0x959d
007818:  0E                           push     cs
007819:  E8 3A 1E                     call     0x9656
00781C:  73 18                        jae      0x7836
00781E:  C7 06 32 0A 02 00            mov      word ptr [0xa32], 2
007824:  81 3E 2A 0A A0 00            cmp      word ptr [0xa2a], 0xa0
00782A:  77 06                        ja       0x7832
00782C:  C7 06 32 0A 03 00            mov      word ptr [0xa32], 3
007832:  0E                           push     cs
007833:  E8 14 1D                     call     0x954a
007836:  F6 06 DA 27 01               test     byte ptr [0x27da], 1
00783B:  74 03                        je       0x7840
00783D:  E8 0E 12                     call     0x8a4e
007840:  E8 CD 1C                     call     0x9510
007843:  B8 00 00                     mov      ax, 0
007846:  BB 1F 00                     mov      bx, 0x1f
007849:  9A 67 14 99 02               lcall    0x299, 0x1467
00784E:  C7 06 49 52 01 00            mov      word ptr [0x5249], 1
007854:  E8 79 00                     call     0x78d0
007857:  E8 21 05                     call     0x7d7b
00785A:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
00785F:  75 23                        jne      0x7884
007861:  F6 06 DA 27 01               test     byte ptr [0x27da], 1
007866:  74 0D                        je       0x7875
007868:  B8 14 00                     mov      ax, 0x14
00786B:  BB 1F 00                     mov      bx, 0x1f
00786E:  9A E1 14 99 02               lcall    0x299, 0x14e1
007873:  EB 0F                        jmp      0x7884
007875:  80 3E 8B 27 00               cmp      byte ptr [0x278b], 0
00787A:  75 08                        jne      0x7884
00787C:  BF 12 66                     mov      di, 0x6612
00787F:  9A 0D 21 99 02               lcall    0x299, 0x210d
007884:  E8 47 14                     call     0x8cce
007887:  E8 A3 00                     call     0x792d
00788A:  E8 58 01                     call     0x79e5
00788D:  F6 06 B8 0D 01               test     byte ptr [0xdb8], 1
007892:  74 35                        je       0x78c9
007894:  B8 01 00                     mov      ax, 1
007897:  BB 13 00                     mov      bx, 0x13
00789A:  9A E1 14 99 02               lcall    0x299, 0x14e1
00789F:  8C E8                        mov      ax, gs
0078A1:  8E D8                        mov      ds, ax
0078A3:  8E C0                        mov      es, ax
0078A5:  E8 03 13                     call     0x8bab
0078A8:  E8 3D 0A                     call     0x82e8
0078AB:  E8 34 0D                     call     0x85e2
0078AE:  F6 06 E5 27 01               test     byte ptr [0x27e5], 1
0078B3:  74 14                        je       0x78c9
0078B5:  BB 89 00                     mov      bx, 0x89
0078B8:  B9 8B 00                     mov      cx, 0x8b
0078BB:  BA 32 00                     mov      dx, 0x32
0078BE:  BD 2C 00                     mov      bp, 0x2c
0078C1:  BE 11 60                     mov      si, 0x6011
0078C4:  9A 0E 04 99 02               lcall    0x299, 0x40e
0078C9:  5D                           pop      bp
0078CA:  5A                           pop      dx
0078CB:  59                           pop      cx
0078CC:  5B                           pop      bx
0078CD:  66 58                        pop      eax
0078CF:  CB                           retf    
