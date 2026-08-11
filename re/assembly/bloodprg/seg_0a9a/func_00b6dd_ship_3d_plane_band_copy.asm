; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b6dd
; seg_off: 0a9a:073d
; group: seg_0a9a
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: ship_3d_plane_band_copy
; label_comment: gated VGA planar page-band copy into framebuffer for ship/procedural-3D path
; incoming: call@0x005c06->0a9a:073d
; byte_count: 127
; boundary: cfg_blocks_7_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 616bbe2388ea24026c85002272ceaa97797ba660dbfa0531582defe5309fbe55

00B6DD:  06                           push     es
00B6DE:  57                           push     di
00B6DF:  1E                           push     ds
00B6E0:  56                           push     si
00B6E1:  50                           push     ax
00B6E2:  53                           push     bx
00B6E3:  51                           push     cx
00B6E4:  52                           push     dx
00B6E5:  F6 06 2E 25 01               test     byte ptr [0x252e], 1
00B6EA:  74 67                        je       0xb753
00B6EC:  8B 1E 27 25                  mov      bx, word ptr [0x2527]
00B6F0:  83 3E 4D 52 0A               cmp      word ptr [0x524d], 0xa
00B6F5:  74 14                        je       0xb70b
00B6F7:  8B C3                        mov      ax, bx
00B6F9:  03 C0                        add      ax, ax
00B6FB:  83 F8 64                     cmp      ax, 0x64
00B6FE:  7E 03                        jle      0xb703
00B700:  B8 64 00                     mov      ax, 0x64
00B703:  83 E8 64                     sub      ax, 0x64
00B706:  F7 D8                        neg      ax
00B708:  A3 4F 52                     mov      word ptr [0x524f], ax
00B70B:  BA C4 03                     mov      dx, 0x3c4
00B70E:  B8 02 0F                     mov      ax, 0xf02
00B711:  EF                           out      dx, ax
00B712:  C4 3E 19 52                  les      di, ptr [0x5219]
00B716:  06                           push     es
00B717:  1F                           pop      ds
00B718:  BE 00 C0                     mov      si, 0xc000
00B71B:  57                           push     di
00B71C:  8B C3                        mov      ax, bx
00B71E:  83 C0 23                     add      ax, 0x23
00B721:  B2 50                        mov      dl, 0x50
00B723:  F6 E2                        mul      dl
00B725:  8B C8                        mov      cx, ax
00B727:  51                           push     cx
00B728:  B8 40 1F                     mov      ax, 0x1f40
00B72B:  2B C1                        sub      ax, cx
00B72D:  03 F0                        add      si, ax
00B72F:  BA CE 03                     mov      dx, 0x3ce
00B732:  B0 05                        mov      al, 5
00B734:  EE                           out      dx, al
00B735:  42                           inc      dx
00B736:  EC                           in       al, dx
00B737:  50                           push     ax
00B738:  24 FC                        and      al, 0xfc
00B73A:  0C 01                        or       al, 1
00B73C:  EE                           out      dx, al
00B73D:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00B73F:  58                           pop      ax
00B740:  59                           pop      cx
00B741:  5F                           pop      di
00B742:  81 C7 40 1F                  add      di, 0x1f40
00B746:  BE 40 DF                     mov      si, 0xdf40
00B749:  BB 40 1F                     mov      bx, 0x1f40
00B74C:  2B D9                        sub      bx, cx
00B74E:  03 FB                        add      di, bx
00B750:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00B752:  EE                           out      dx, al
00B753:  5A                           pop      dx
00B754:  59                           pop      cx
00B755:  5B                           pop      bx
00B756:  58                           pop      ax
00B757:  5E                           pop      si
00B758:  1F                           pop      ds
00B759:  5F                           pop      di
00B75A:  07                           pop      es
00B75B:  CB                           retf    
