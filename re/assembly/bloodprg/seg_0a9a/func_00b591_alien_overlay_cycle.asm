; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b591
; seg_off: 0a9a:05f1
; group: seg_0a9a
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: alien_overlay_cycle
; label_comment: the {amer, croolis, scrut}.xdb overlay CYCLE (name table DS:0xACC, index [0xAE5] 0..2) with the cursor pushed around the load/call — the ALIEN-EXAMINATION screen path (validates the port's alien_view cycling). NOT the hand's per-frame caller (still pinned: the manu3 segment-storage consumer) || ALSO RECORDED as `ship_3d_temp_snd_setup`: temporary sn\3D.snd presentation path; cycles DS:0x0AE5 and restores sn\tb.snd || RESOLVED 2026-07-25 (#186): both readings are HALVES of one routine, and both are right. DS:0x0ACC is a 3-pointer table reading amer.xdb / croolis.xdb / scrut.xdb, cycled 0..2 by [0x0AE5] (inc ah / cmp ah,3 / jne / xor ah,ah at 0xB5B5); the same routine then loads DS:0x0D23 = 'sn\\3D.snd' via lcall 0xB1B:0x855. It cycles the alien overlay AND swaps the SND bank || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; incoming: call@0x0019df->0a9a:05f1
; byte_count: 257
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0xb690:1, retf:1
; direct_callees: none
; indirect_calls: 11
; routine_bytes_sha256: 7abf19b449320cd3b3a67b20979c0b29faf4fdf13dfa3ff5ba22cebdbc7094c3

00B591:  06                           push     es
00B592:  F6 06 E4 0A 01               test     byte ptr [0xae4], 1
00B597:  0F 84 F5 00                  je       0xb690
00B59B:  C6 06 E4 0A 00               mov      byte ptr [0xae4], 0
00B5A0:  C6 06 E3 0A 00               mov      byte ptr [0xae3], 0
00B5A5:  FF 36 2A 0A                  push     word ptr [0xa2a]
00B5A9:  FF 36 2C 0A                  push     word ptr [0xa2c]
00B5AD:  BE CC 0A                     mov      si, 0xacc
00B5B0:  A0 E5 0A                     mov      al, byte ptr [0xae5]
00B5B3:  8A E0                        mov      ah, al
00B5B5:  FE C4                        inc      ah
00B5B7:  80 FC 03                     cmp      ah, 3
00B5BA:  75 02                        jne      0xb5be
00B5BC:  32 E4                        xor      ah, ah
00B5BE:  88 26 E5 0A                  mov      byte ptr [0xae5], ah
00B5C2:  98                           cwde    
00B5C3:  03 C0                        add      ax, ax
00B5C5:  03 F0                        add      si, ax
00B5C7:  8B 34                        mov      si, word ptr [si]
00B5C9:  C4 3E 96 0A                  les      di, ptr [0xa96]
00B5CD:  9A DB 07 CE 01               lcall    0x1ce, 0x7db
00B5D2:  66 FF 36 BB 0B               push     dword ptr [0xbbb]
00B5D7:  BE 23 0D                     mov      si, 0xd23
00B5DA:  33 C0                        xor      ax, ax
00B5DC:  9A 55 08 1B 0B               lcall    0xb1b, 0x855
00B5E1:  BD E8 0A                     mov      bp, 0xae8
00B5E4:  A1 9C 67                     mov      ax, word ptr [0x679c]
00B5E7:  89 46 00                     mov      word ptr [bp], ax
00B5EA:  A1 26 67                     mov      ax, word ptr [0x6726]
00B5ED:  89 46 02                     mov      word ptr [bp + 2], ax
00B5F0:  FF 36 A0 0B                  push     word ptr [0xba0]
00B5F4:  C6 06 A0 0B 00               mov      byte ptr [0xba0], 0
00B5F9:  9A 14 05 8B 00               lcall    0x8b, 0x514
00B5FE:  FF 1E 96 0A                  lcall    [0xa96]
00B602:  9A E7 04 8B 00               lcall    0x8b, 0x4e7
00B607:  8F 06 A0 0B                  pop      word ptr [0xba0]
00B60B:  BE FC 0C                     mov      si, 0xcfc
00B60E:  33 C0                        xor      ax, ax
00B610:  9A 55 08 1B 0B               lcall    0xb1b, 0x855
00B615:  66 8F 06 BB 0B               pop      dword ptr [0xbbb]
00B61A:  BE 13 01                     mov      si, 0x113
00B61D:  9A DB 07 CE 01               lcall    0x1ce, 0x7db
00B622:  33 C0                        xor      ax, ax
00B624:  9A EB 0D 99 02               lcall    0x299, 0xdeb
00B629:  C4 3E 2D 52                  les      di, ptr [0x522d]
00B62D:  66 33 C0                     xor      eax, eax
00B630:  AB                           stosw    word ptr es:[di], ax
00B631:  40                           inc      ax
00B632:  AB                           stosw    word ptr es:[di], ax
00B633:  83 C0 03                     add      ax, 3
00B636:  66 AB                        stosd    dword ptr es:[di], eax
00B638:  B8 40 01                     mov      ax, 0x140
00B63B:  AB                           stosw    word ptr es:[di], ax
00B63C:  B8 C8 00                     mov      ax, 0xc8
00B63F:  AB                           stosw    word ptr es:[di], ax
00B640:  33 C0                        xor      ax, ax
00B642:  66 AB                        stosd    dword ptr es:[di], eax
00B644:  C7 06 3B 0B 00 00            mov      word ptr [0xb3b], 0
00B64A:  C6 06 55 5B 01               mov      byte ptr [0x5b55], 1
00B64F:  8F 06 2C 0A                  pop      word ptr [0xa2c]
00B653:  8F 06 2A 0A                  pop      word ptr [0xa2a]
00B657:  F6 06 2A 25 01               test     byte ptr [0x252a], 1
00B65C:  74 17                        je       0xb675
00B65E:  C6 06 2E 25 00               mov      byte ptr [0x252e], 0
00B663:  9A 67 09 8B 00               lcall    0x8b, 0x967
00B668:  C6 06 2E 25 01               mov      byte ptr [0x252e], 1
00B66D:  C7 06 A3 1F FF FF            mov      word ptr [0x1fa3], 0xffff
00B673:  EB 1B                        jmp      0xb690
00B675:  9A 29 09 8B 00               lcall    0x8b, 0x929
00B67A:  BE F3 00                     mov      si, 0xf3
00B67D:  C4 3E 29 52                  les      di, ptr [0x5229]
00B681:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
00B686:  C6 06 57 5B 00               mov      byte ptr [0x5b57], 0
00B68B:  9A 1D 09 CE 01               lcall    0x1ce, 0x91d
00B690:  07                           pop      es
00B691:  CB                           retf    
