; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b75c
; seg_off: 0a9a:07bc
; group: seg_0a9a
; provenance: recursive_graph
; label: ship_3d_depth_scroll_step
; label_comment: moves DS:0x2527 toward the active target using step DS:0x2531
; byte_count: 76
; boundary: cfg_blocks_13_terminals_4
; terminal: jmp 0xb7a5:3, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 7b169cde9fa6c63a0388539519b45c9d087b3079b8bfc60fe27c28ade04553dd

00B75C:  50                           push     ax
00B75D:  53                           push     bx
00B75E:  F6 06 2F 25 01               test     byte ptr [0x252f], 1
00B763:  74 20                        je       0xb785
00B765:  A1 27 25                     mov      ax, word ptr [0x2527]
00B768:  83 F8 41                     cmp      ax, 0x41
00B76B:  74 11                        je       0xb77e
00B76D:  02 06 31 25                  add      al, byte ptr [0x2531]
00B771:  83 F8 41                     cmp      ax, 0x41
00B774:  7C 03                        jl       0xb779
00B776:  B8 41 00                     mov      ax, 0x41
00B779:  A3 27 25                     mov      word ptr [0x2527], ax
00B77C:  EB 27                        jmp      0xb7a5
00B77E:  C6 06 2F 25 00               mov      byte ptr [0x252f], 0
00B783:  EB 20                        jmp      0xb7a5
00B785:  F6 06 30 25 01               test     byte ptr [0x2530], 1
00B78A:  74 19                        je       0xb7a5
00B78C:  A1 27 25                     mov      ax, word ptr [0x2527]
00B78F:  0B C0                        or       ax, ax
00B791:  74 0D                        je       0xb7a0
00B793:  2A 06 31 25                  sub      al, byte ptr [0x2531]
00B797:  79 02                        jns      0xb79b
00B799:  33 C0                        xor      ax, ax
00B79B:  A3 27 25                     mov      word ptr [0x2527], ax
00B79E:  EB 05                        jmp      0xb7a5
00B7A0:  C6 06 30 25 00               mov      byte ptr [0x2530], 0
00B7A5:  5B                           pop      bx
00B7A6:  58                           pop      ax
00B7A7:  C3                           ret     
