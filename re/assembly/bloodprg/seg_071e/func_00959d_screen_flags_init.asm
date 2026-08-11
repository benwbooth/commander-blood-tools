; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00959d
; seg_off: 071e:1dbd
; group: seg_071e
; provenance: recursive_graph
; label: screen_flags_init
; label_comment: state-init helper (4 calls): [0x27d9]=0; [0x5b53]=1; [0x5b55]=1. Sets screen/presentation-ready flags - part of a mode/screen setup sequence
; byte_count: 162
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x95fa:1, ret:1
; direct_callees: 0x00954a, 0x00963f, 0x00981b
; indirect_calls: 5
; routine_bytes_sha256: 7149435d9dd1aa13cc8fcae0692e9f8b70745c16634d981a16c0d143a078d1b2

00959D:  55                           push     bp
00959E:  53                           push     bx
00959F:  51                           push     cx
0095A0:  52                           push     dx
0095A1:  56                           push     si
0095A2:  57                           push     di
0095A3:  C6 06 D9 27 00               mov      byte ptr [0x27d9], 0
0095A8:  C6 06 53 5B 01               mov      byte ptr [0x5b53], 1
0095AD:  C6 06 55 5B 01               mov      byte ptr [0x5b55], 1
0095B2:  C6 06 E5 27 00               mov      byte ptr [0x27e5], 0
0095B7:  C7 06 49 52 01 00            mov      word ptr [0x5249], 1
0095BD:  F6 06 DA 27 01               test     byte ptr [0x27da], 1
0095C2:  74 2A                        je       0x95ee
0095C4:  33 C0                        xor      ax, ax
0095C6:  A2 57 5B                     mov      byte ptr [0x5b57], al
0095C9:  A2 31 52                     mov      byte ptr [0x5231], al
0095CC:  A1 95 27                     mov      ax, word ptr [0x2795]
0095CF:  E8 49 02                     call     0x981b
0095D2:  33 C0                        xor      ax, ax
0095D4:  9A EB 0D 99 02               lcall    0x299, 0xdeb
0095D9:  33 C0                        xor      ax, ax
0095DB:  8B D8                        mov      bx, ax
0095DD:  8B C8                        mov      cx, ax
0095DF:  8B E8                        mov      bp, ax
0095E1:  BA 0B 00                     mov      dx, 0xb
0095E4:  B8 14 00                     mov      ax, 0x14
0095E7:  9A 40 11 99 02               lcall    0x299, 0x1140
0095EC:  EB 0C                        jmp      0x95fa
0095EE:  0E                           push     cs
0095EF:  E8 58 FF                     call     0x954a
0095F2:  B8 04 00                     mov      ax, 4
0095F5:  9A 41 12 99 02               lcall    0x299, 0x1241
0095FA:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
0095FF:  C7 06 27 25 00 00            mov      word ptr [0x2527], 0
009605:  B9 C0 00                     mov      cx, 0xc0
009608:  BE 58 5B                     mov      si, 0x5b58
00960B:  BF 51 52                     mov      di, 0x5251
00960E:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
009611:  B8 CE FF                     mov      ax, 0xffce
009614:  33 DB                        xor      bx, bx
009616:  8B CB                        mov      cx, bx
009618:  8B D3                        mov      dx, bx
00961A:  BF 11 5F                     mov      di, 0x5f11
00961D:  9A 00 00 CE 01               lcall    0x1ce, 0
009622:  B8 E0 00                     mov      ax, 0xe0
009625:  BB 11 60                     mov      bx, 0x6011
009628:  9A 4D 01 CE 01               lcall    0x1ce, 0x14d
00962D:  F6 06 E0 27 01               test     byte ptr [0x27e0], 1
009632:  75 04                        jne      0x9638
009634:  0E                           push     cs
009635:  E8 07 00                     call     0x963f
009638:  5F                           pop      di
009639:  5E                           pop      si
00963A:  5A                           pop      dx
00963B:  59                           pop      cx
00963C:  5B                           pop      bx
00963D:  5D                           pop      bp
00963E:  C3                           ret     
