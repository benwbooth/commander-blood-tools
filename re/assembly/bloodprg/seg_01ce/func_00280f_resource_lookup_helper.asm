; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00280f
; seg_off: 01ce:052f
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_lookup_helper
; label_comment: resource lookup helper: push cs; call resource_name_lookup 0x28ca; ebp result. Resolves a resource by its name-table entry (used in the load path)
; incoming: call@0x001779->01ce:052f
; byte_count: 108
; boundary: cfg_blocks_7_terminals_1
; terminal: retf:1
; direct_callees: 0x0028ca
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_00280f_resource_lookup_helper.cpp
; routine_bytes_sha256: 00a0b6c3da48f564e7d4ae546eca794abaddd154d69232c60a802a94bc440ca6

00280F:  66 55                        push     ebp
002811:  1E                           push     ds
002812:  66 50                        push     eax
002814:  51                           push     cx
002815:  53                           push     bx
002816:  52                           push     dx
002817:  66 33 C0                     xor      eax, eax
00281A:  0E                           push     cs
00281B:  E8 AC 00                     call     0x28ca
00281E:  66 0B ED                     or       ebp, ebp
002821:  74 4F                        je       0x2872
002823:  8B D6                        mov      dx, si
002825:  B8 00 3D                     mov      ax, 0x3d00
002828:  CD 21                        int      0x21
00282A:  72 46                        jb       0x2872
00282C:  65 A3 84 0A                  mov      word ptr gs:[0xa84], ax
002830:  8B D7                        mov      dx, di
002832:  33 C9                        xor      cx, cx
002834:  B8 00 3C                     mov      ax, 0x3c00
002837:  CD 21                        int      0x21
002839:  72 37                        jb       0x2872
00283B:  8B D8                        mov      bx, ax
00283D:  65 C5 16 7C 0A               lds      dx, ptr gs:[0xa7c]
002842:  65 87 1E 84 0A               xchg     word ptr gs:[0xa84], bx
002847:  B9 00 FA                     mov      cx, 0xfa00
00284A:  B8 00 3F                     mov      ax, 0x3f00
00284D:  CD 21                        int      0x21
00284F:  66 2B E8                     sub      ebp, eax
002852:  65 87 1E 84 0A               xchg     word ptr gs:[0xa84], bx
002857:  8B C8                        mov      cx, ax
002859:  B8 00 40                     mov      ax, 0x4000
00285C:  CD 21                        int      0x21
00285E:  66 0B ED                     or       ebp, ebp
002861:  75 DF                        jne      0x2842
002863:  B8 00 3E                     mov      ax, 0x3e00
002866:  CD 21                        int      0x21
002868:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
00286D:  B8 00 3E                     mov      ax, 0x3e00
002870:  CD 21                        int      0x21
002872:  5A                           pop      dx
002873:  5B                           pop      bx
002874:  59                           pop      cx
002875:  66 58                        pop      eax
002877:  1F                           pop      ds
002878:  66 5D                        pop      ebp
00287A:  CB                           retf    
