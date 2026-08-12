; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a240
; seg_off: 0971:0530
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_advance_due
; label_comment: gate active D8C queue advancement by audio playback position when presentation/audio bits are all set, otherwise by the software tick threshold; carry clear means due and carry set means wait
; byte_count: 81
; boundary: cfg_blocks_13_terminals_2
; terminal: jmp 0xa290:1, ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 13d0c4bc7f9892797b231cf4df9424aa2f9d30bf34791cf7cf98e351beed0b73

00A240:  F6 06 E0 27 01               test     byte ptr [0x27e0], 1
00A245:  74 2D                        je       0xa274
00A247:  F6 06 E1 27 01               test     byte ptr [0x27e1], 1
00A24C:  74 26                        je       0xa274
00A24E:  F6 06 DE 0A 01               test     byte ptr [0xade], 1
00A253:  74 1F                        je       0xa274
00A255:  FF 1E F3 0C                  lcall    [0xcf3]
00A259:  2D 00 40                     sub      ax, 0x4000
00A25C:  F7 D8                        neg      ax
00A25E:  50                           push     ax
00A25F:  2B 06 41 0C                  sub      ax, word ptr [0xc41]
00A263:  79 03                        jns      0xa268
00A265:  05 00 40                     add      ax, 0x4000
00A268:  3D 98 03                     cmp      ax, 0x398
00A26B:  58                           pop      ax
00A26C:  72 22                        jb       0xa290
00A26E:  A3 41 0C                     mov      word ptr [0xc41], ax
00A271:  F8                           clc
00A272:  EB 1C                        jmp      0xa290
00A274:  A1 29 0B                     mov      ax, word ptr [0xb29]
00A277:  2B 06 A2 0D                  sub      ax, word ptr [0xda2]
00A27B:  79 02                        jns      0xa27f
00A27D:  F7 D8                        neg      ax
00A27F:  0A E4                        or       ah, ah
00A281:  75 06                        jne      0xa289
00A283:  3A 06 77 0D                  cmp      al, byte ptr [0xd77]
00A287:  72 07                        jb       0xa290
00A289:  A1 29 0B                     mov      ax, word ptr [0xb29]
00A28C:  A3 A2 0D                     mov      word ptr [0xda2], ax
00A28F:  F8                           clc
00A290:  C3                           ret
