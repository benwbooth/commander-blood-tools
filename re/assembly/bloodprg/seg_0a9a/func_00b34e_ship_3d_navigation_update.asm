; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b34e
; seg_off: 0a9a:03ae
; group: seg_0a9a
; provenance: recursive_graph
; label: ship_3d_navigation_update
; label_comment: ship/navigation update branch gated by DS:0x27D8; updates target records and transition flags
; byte_count: 579
; boundary: cfg_blocks_25_terminals_4
; terminal: jmp 0xb3f0:1, jmp 0xb58c:2, ret:1
; direct_callees: 0x00b591
; indirect_calls: 14
; routine_bytes_sha256: 139b65834a72050bebc832debed5bb873478bcca4760ed8afa6c4d1da8270137

00B34E:  66 50                        push     eax
00B350:  53                           push     bx
00B351:  51                           push     cx
00B352:  F6 06 D8 27 01               test     byte ptr [0x27d8], 1
00B357:  0F 84 26 01                  je       0xb481
00B35B:  A1 36 0A                     mov      ax, word ptr [0xa36]
00B35E:  A3 32 0A                     mov      word ptr [0xa32], ax
00B361:  06                           push     es
00B362:  8E 06 26 67                  mov      es, word ptr [0x6726]
00B366:  8B 3E 1B 25                  mov      di, word ptr [0x251b]
00B36A:  57                           push     di
00B36B:  26 F7 05 80 00               test     word ptr es:[di], 0x80
00B370:  74 04                        je       0xb376
00B372:  26 8B 7D 14                  mov      di, word ptr es:[di + 0x14]
00B376:  26 FF 45 14                  inc      word ptr es:[di + 0x14]
00B37A:  5F                           pop      di
00B37B:  9A 4E 1D DA 04               lcall    0x4da, 0x1d4e
00B380:  BD 53 2B                     mov      bp, 0x2b53
00B383:  8B 46 00                     mov      ax, word ptr [bp]
00B386:  83 C5 02                     add      bp, 2
00B389:  0B C0                        or       ax, ax
00B38B:  74 36                        je       0xb3c3
00B38D:  26 F6 45 02 02               test     byte ptr es:[di + 2], 2
00B392:  75 08                        jne      0xb39c
00B394:  8B D8                        mov      bx, ax
00B396:  26 3B 7F 18                  cmp      di, word ptr es:[bx + 0x18]
00B39A:  75 E7                        jne      0xb383
00B39C:  8B 1E 58 67                  mov      bx, word ptr [0x6758]
00B3A0:  3B 1E 1B 25                  cmp      bx, word ptr [0x251b]
00B3A4:  74 08                        je       0xb3ae
00B3A6:  8B F8                        mov      di, ax
00B3A8:  26 3B 5D 18                  cmp      bx, word ptr es:[di + 0x18]
00B3AC:  74 15                        je       0xb3c3
00B3AE:  C7 06 68 67 C4 00            mov      word ptr [0x6768], 0xc4
00B3B4:  A3 6A 67                     mov      word ptr [0x676a], ax
00B3B7:  83 C0 04                     add      ax, 4
00B3BA:  8B F8                        mov      di, ax
00B3BC:  9A 69 20 DA 04               lcall    0x4da, 0x2069
00B3C1:  EB 2D                        jmp      0xb3f0
00B3C3:  80 0E 93 27 04               or       byte ptr [0x2793], 4
00B3C8:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
00B3CD:  C6 06 DA 0A 06               mov      byte ptr [0xada], 6
00B3D2:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
00B3D7:  BE 3B 25                     mov      si, 0x253b
00B3DA:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
00B3DF:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
00B3E4:  A1 AB 2A                     mov      ax, word ptr [0x2aab]
00B3E7:  A3 4D 25                     mov      word ptr [0x254d], ax
00B3EA:  A1 AF 2A                     mov      ax, word ptr [0x2aaf]
00B3ED:  A3 51 25                     mov      word ptr [0x2551], ax
00B3F0:  C6 06 D8 27 00               mov      byte ptr [0x27d8], 0
00B3F5:  C6 06 2A 25 01               mov      byte ptr [0x252a], 1
00B3FA:  C7 06 A7 1F 23 00            mov      word ptr [0x1fa7], 0x23
00B400:  C7 06 A3 1F FF FF            mov      word ptr [0x1fa3], 0xffff
00B406:  07                           pop      es
00B407:  C7 06 39 52 23 00            mov      word ptr [0x5239], 0x23
00B40D:  C7 06 3B 52 A5 00            mov      word ptr [0x523b], 0xa5
00B413:  33 C0                        xor      ax, ax
00B415:  9A 2F 0E 99 02               lcall    0x299, 0xe2f
00B41A:  A3 39 52                     mov      word ptr [0x5239], ax
00B41D:  C7 06 3B 52 C8 00            mov      word ptr [0x523b], 0xc8
00B423:  BE D7 0D                     mov      si, 0xdd7
00B426:  C6 06 53 5B 01               mov      byte ptr [0x5b53], 1
00B42B:  C6 06 57 5B 01               mov      byte ptr [0x5b57], 1
00B430:  06                           push     es
00B431:  C4 3E 29 52                  les      di, ptr [0x5229]
00B435:  C6 06 E1 0A 01               mov      byte ptr [0xae1], 1
00B43A:  9A 1D 09 CE 01               lcall    0x1ce, 0x91d
00B43F:  C6 06 E1 0A 00               mov      byte ptr [0xae1], 0
00B444:  07                           pop      es
00B445:  BE D1 53                     mov      si, 0x53d1
00B448:  BF D1 59                     mov      di, 0x59d1
00B44B:  B9 30 00                     mov      cx, 0x30
00B44E:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00B451:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
00B456:  C6 06 57 5B 00               mov      byte ptr [0x5b57], 0
00B45B:  C6 06 B3 1F 00               mov      byte ptr [0x1fb3], 0
00B460:  C7 06 AB 1F FF FF            mov      word ptr [0x1fab], 0xffff
00B466:  C6 06 30 25 01               mov      byte ptr [0x2530], 1
00B46B:  C6 06 31 25 02               mov      byte ptr [0x2531], 2
00B470:  BF 11 5F                     mov      di, 0x5f11
00B473:  B8 CE FF                     mov      ax, 0xffce
00B476:  33 DB                        xor      bx, bx
00B478:  8B CB                        mov      cx, bx
00B47A:  8B D3                        mov      dx, bx
00B47C:  9A 00 00 CE 01               lcall    0x1ce, 0
00B481:  F6 06 32 25 01               test     byte ptr [0x2532], 1
00B486:  75 6A                        jne      0xb4f2
00B488:  F6 06 2A 25 01               test     byte ptr [0x252a], 1
00B48D:  74 4D                        je       0xb4dc
00B48F:  0E                           push     cs
00B490:  E8 FE 00                     call     0xb591
00B493:  9A 76 1E 1E 07               lcall    0x71e, 0x1e76
00B498:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
00B49D:  75 3A                        jne      0xb4d9
00B49F:  1E                           push     ds
00B4A0:  C5 36 29 52                  lds      si, ptr [0x5229]
00B4A4:  9A B6 0E 99 02               lcall    0x299, 0xeb6
00B4A9:  1F                           pop      ds
00B4AA:  C6 06 B8 0D 01               mov      byte ptr [0xdb8], 1
00B4AF:  80 3E DA 0A 06               cmp      byte ptr [0xada], 6
00B4B4:  75 23                        jne      0xb4d9
00B4B6:  BE AB 2A                     mov      si, 0x2aab
00B4B9:  BF 4D 25                     mov      di, 0x254d
00B4BC:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
00B4C1:  73 16                        jae      0xb4d9
00B4C3:  BE 3B 25                     mov      si, 0x253b
00B4C6:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
00B4CB:  0B C0                        or       ax, ax
00B4CD:  78 0A                        js       0xb4d9
00B4CF:  C6 06 2A 25 00               mov      byte ptr [0x252a], 0
00B4D4:  C6 06 32 25 01               mov      byte ptr [0x2532], 1
00B4D9:  E9 B0 00                     jmp      0xb58c
00B4DC:  F6 06 B0 67 01               test     byte ptr [0x67b0], 1
00B4E1:  0F 85 A7 00                  jne      0xb58c
00B4E5:  C6 06 32 25 01               mov      byte ptr [0x2532], 1
00B4EA:  C6 06 2F 25 01               mov      byte ptr [0x252f], 1
00B4EF:  E9 9A 00                     jmp      0xb58c
00B4F2:  F6 06 2F 25 01               test     byte ptr [0x252f], 1
00B4F7:  75 96                        jne      0xb48f
00B4F9:  33 C0                        xor      ax, ax
00B4FB:  9A EB 0D 99 02               lcall    0x299, 0xdeb
00B500:  9A AB 01 CE 01               lcall    0x1ce, 0x1ab
00B505:  C7 06 93 27 09 00            mov      word ptr [0x2793], 9
00B50B:  C7 06 9B 27 00 00            mov      word ptr [0x279b], 0
00B511:  C7 06 9D 27 32 00            mov      word ptr [0x279d], 0x32
00B517:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
00B51C:  C6 06 39 27 01               mov      byte ptr [0x2739], 1
00B521:  33 C0                        xor      ax, ax
00B523:  A3 F3 24                     mov      word ptr [0x24f3], ax
00B526:  A3 A7 1F                     mov      word ptr [0x1fa7], ax
00B529:  C7 06 AB 1F FF FF            mov      word ptr [0x1fab], 0xffff
00B52F:  C7 06 88 67 FF FF            mov      word ptr [0x6788], 0xffff
00B535:  A2 B2 1F                     mov      byte ptr [0x1fb2], al
00B538:  A2 32 25                     mov      byte ptr [0x2532], al
00B53B:  A2 29 25                     mov      byte ptr [0x2529], al
00B53E:  A2 64 5E                     mov      byte ptr [0x5e64], al
00B541:  A2 B0 67                     mov      byte ptr [0x67b0], al
00B544:  A2 BC 67                     mov      byte ptr [0x67bc], al
00B547:  A2 2E 25                     mov      byte ptr [0x252e], al
00B54A:  A2 2A 25                     mov      byte ptr [0x252a], al
00B54D:  80 26 AA 67 FC               and      byte ptr [0x67aa], 0xfc
00B552:  A2 BA 67                     mov      byte ptr [0x67ba], al
00B555:  9A 29 09 8B 00               lcall    0x8b, 0x929
00B55A:  9A B6 14 1E 07               lcall    0x71e, 0x14b6
00B55F:  8C E8                        mov      ax, gs
00B561:  8E C0                        mov      es, ax
00B563:  BE 58 5B                     mov      si, 0x5b58
00B566:  BF 51 58                     mov      di, 0x5851
00B569:  B9 90 00                     mov      cx, 0x90
00B56C:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00B56F:  BF 51 55                     mov      di, 0x5551
00B572:  66 33 C0                     xor      eax, eax
00B575:  B9 C0 00                     mov      cx, 0xc0
00B578:  F3 66 AB                     rep stosd dword ptr es:[di], eax
00B57B:  C6 06 52 5B FF               mov      byte ptr [0x5b52], 0xff
00B580:  C7 06 4F 52 00 00            mov      word ptr [0x524f], 0
00B586:  C7 06 4D 52 0A 00            mov      word ptr [0x524d], 0xa
00B58C:  59                           pop      cx
00B58D:  5B                           pop      bx
00B58E:  66 58                        pop      eax
00B590:  C3                           ret     
