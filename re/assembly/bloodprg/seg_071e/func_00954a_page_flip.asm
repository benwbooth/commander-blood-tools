; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00954a
; seg_off: 071e:1d6a
; group: seg_071e
; provenance: recursive_graph
; label: page_flip
; label_comment: page-flip / display-swap (4 calls): [0x5b55]=1; save [0x5221]; copy the linear back-buffer far ptr [0x5229] into the display far ptr [0x5221]; lcall 0x299:0xdeb (present) + call 0x98b9. Swaps the composed back buffer to the visible page
; byte_count: 83
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: 0x00981b, 0x0098b9, 0x009a10, 0x009b98
; indirect_calls: 3
; routine_bytes_sha256: 0a61a944d558d1857c447eea7994d4773fcb3597a0e30041fdb6e259936db9c3

00954A:  C6 06 55 5B 01               mov      byte ptr [0x5b55], 1
00954F:  66 FF 36 21 52               push     dword ptr [0x5221]
009554:  66 A1 29 52                  mov      eax, dword ptr [0x5229]
009558:  66 A3 21 52                  mov      dword ptr [0x5221], eax
00955C:  33 C0                        xor      ax, ax
00955E:  9A EB 0D 99 02               lcall    0x299, 0xdeb
009563:  0E                           push     cs
009564:  E8 52 03                     call     0x98b9
009567:  0E                           push     cs
009568:  E8 A5 04                     call     0x9a10
00956B:  0E                           push     cs
00956C:  E8 29 06                     call     0x9b98
00956F:  B8 15 00                     mov      ax, 0x15
009572:  BB 1F 00                     mov      bx, 0x1f
009575:  9A 67 14 99 02               lcall    0x299, 0x1467
00957A:  9A E1 14 99 02               lcall    0x299, 0x14e1
00957F:  66 8F 06 21 52               pop      dword ptr [0x5221]
009584:  F7 06 F3 24 01 00            test     word ptr [0x24f3], 1
00958A:  75 10                        jne      0x959c
00958C:  C6 06 57 5B 01               mov      byte ptr [0x5b57], 1
009591:  C6 06 31 52 01               mov      byte ptr [0x5231], 1
009596:  A1 95 27                     mov      ax, word ptr [0x2795]
009599:  E8 7F 02                     call     0x981b
00959C:  CB                           retf    
