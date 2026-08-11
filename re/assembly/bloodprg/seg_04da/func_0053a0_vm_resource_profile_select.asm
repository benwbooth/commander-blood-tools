; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0053a0
; seg_off: 04da:0000
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target, static_dispatch_table_target
; label: vm_resource_profile_select
; label_comment: select script/resource profile AX; free old DS:0x6712 resources, copy five FS:0x11f4+AX*10 offsets into DS:0x6712, clear VM globals
; incoming: call@0x0010c8->04da:0000
; incoming: call@0x001cf5->04da:0000
; incoming: vm_opcode_handlers:opcode_0xd3
; byte_count: 443
; boundary: cfg_blocks_28_terminals_9
; terminal: jmp 0x5486:1, jmp 0x5524:5, jmp 0x552a:1, jmp 0x5553:1, retf:1
; direct_callees: none
; indirect_calls: 12
; routine_bytes_sha256: ef4501eb798abb40d90c8a77ea8eadec4fa2060f438aa47e481ef830f86b932c

0053A0:  53                           push     bx
0053A1:  51                           push     cx
0053A2:  52                           push     dx
0053A3:  06                           push     es
0053A4:  57                           push     di
0053A5:  1E                           push     ds
0053A6:  56                           push     si
0053A7:  3B 06 7E 67                  cmp      ax, word ptr [0x677e]
0053AB:  74 10                        je       0x53bd
0053AD:  50                           push     ax
0053AE:  B9 05 00                     mov      cx, 5
0053B1:  BE 12 67                     mov      si, 0x6712
0053B4:  AD                           lodsw    ax, word ptr [si]
0053B5:  9A F8 00 B9 04               lcall    0x4b9, 0xf8
0053BA:  E2 F8                        loop     0x53b4
0053BC:  58                           pop      ax
0053BD:  A3 7E 67                     mov      word ptr [0x677e], ax
0053C0:  8C E3                        mov      bx, fs
0053C2:  8E DB                        mov      ds, bx
0053C4:  8C EB                        mov      bx, gs
0053C6:  8E C3                        mov      es, bx
0053C8:  BF 12 67                     mov      di, 0x6712
0053CB:  BE F4 11                     mov      si, 0x11f4
0053CE:  BA 0A 00                     mov      dx, 0xa
0053D1:  F7 E2                        mul      dx
0053D3:  03 F0                        add      si, ax
0053D5:  8B DE                        mov      bx, si
0053D7:  B9 05 00                     mov      cx, 5
0053DA:  AD                           lodsw    ax, word ptr [si]
0053DB:  AB                           stosw    word ptr es:[di], ax
0053DC:  9A 9B 05 CE 01               lcall    0x1ce, 0x59b
0053E1:  0B C0                        or       ax, ax
0053E3:  0F 84 69 01                  je       0x5550
0053E7:  E2 F1                        loop     0x53da
0053E9:  8C E8                        mov      ax, gs
0053EB:  8E D8                        mov      ds, ax
0053ED:  B9 40 00                     mov      cx, 0x40
0053F0:  66 B8 FF FF FF FF            mov      eax, 0xffffffff
0053F6:  BF DE 6A                     mov      di, 0x6ade
0053F9:  F3 66 AB                     rep stosd dword ptr es:[di], eax
0053FC:  66 33 C0                     xor      eax, eax
0053FF:  BF 3E 6D                     mov      di, 0x6d3e
005402:  B9 10 00                     mov      cx, 0x10
005405:  F3 AB                        rep stosw word ptr es:[di], ax
005407:  A3 30 67                     mov      word ptr [0x6730], ax
00540A:  A3 84 68                     mov      word ptr [0x6884], ax
00540D:  A2 AD 67                     mov      byte ptr [0x67ad], al
005410:  A3 32 67                     mov      word ptr [0x6732], ax
005413:  A3 5A 67                     mov      word ptr [0x675a], ax
005416:  A2 AA 67                     mov      byte ptr [0x67aa], al
005419:  A2 A8 67                     mov      byte ptr [0x67a8], al
00541C:  A3 44 67                     mov      word ptr [0x6744], ax
00541F:  A2 AB 67                     mov      byte ptr [0x67ab], al
005422:  A2 AC 67                     mov      byte ptr [0x67ac], al
005425:  A2 B0 67                     mov      byte ptr [0x67b0], al
005428:  A2 64 5E                     mov      byte ptr [0x5e64], al
00542B:  A2 D7 27                     mov      byte ptr [0x27d7], al
00542E:  A3 62 67                     mov      word ptr [0x6762], ax
005431:  A3 66 67                     mov      word ptr [0x6766], ax
005434:  A3 68 67                     mov      word ptr [0x6768], ax
005437:  A3 6A 67                     mov      word ptr [0x676a], ax
00543A:  A3 6E 67                     mov      word ptr [0x676e], ax
00543D:  A2 AE 67                     mov      byte ptr [0x67ae], al
005440:  A2 B7 67                     mov      byte ptr [0x67b7], al
005443:  A2 AF 67                     mov      byte ptr [0x67af], al
005446:  A2 B2 67                     mov      byte ptr [0x67b2], al
005449:  A2 B1 67                     mov      byte ptr [0x67b1], al
00544C:  A3 7A 67                     mov      word ptr [0x677a], ax
00544F:  A3 78 67                     mov      word ptr [0x6778], ax
005452:  A3 86 68                     mov      word ptr [0x6886], ax
005455:  A3 70 67                     mov      word ptr [0x6770], ax
005458:  A3 86 67                     mov      word ptr [0x6786], ax
00545B:  A3 82 67                     mov      word ptr [0x6782], ax
00545E:  A3 84 67                     mov      word ptr [0x6784], ax
005461:  A3 72 67                     mov      word ptr [0x6772], ax
005464:  A3 74 67                     mov      word ptr [0x6774], ax
005467:  A3 34 67                     mov      word ptr [0x6734], ax
00546A:  A3 36 67                     mov      word ptr [0x6736], ax
00546D:  A3 A0 67                     mov      word ptr [0x67a0], ax
005470:  A3 A2 67                     mov      word ptr [0x67a2], ax
005473:  A1 16 67                     mov      ax, word ptr [0x6716]
005476:  9A 90 01 B9 04               lcall    0x4b9, 0x190
00547B:  8C DB                        mov      bx, ds
00547D:  65 A1 1A 67                  mov      ax, word ptr gs:[0x671a]
005481:  9A 90 01 B9 04               lcall    0x4b9, 0x190
005486:  8B 44 12                     mov      ax, word ptr [si + 0x12]
005489:  83 F8 01                     cmp      ax, 1
00548C:  0F 85 9A 00                  jne      0x552a
005490:  BF BE 67                     mov      di, 0x67be
005493:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
005498:  73 1C                        jae      0x54b6
00549A:  8B 44 10                     mov      ax, word ptr [si + 0x10]
00549D:  65 A3 4E 67                  mov      word ptr gs:[0x674e], ax
0054A1:  83 C0 08                     add      ax, 8
0054A4:  65 A3 5E 67                  mov      word ptr gs:[0x675e], ax
0054A8:  83 C0 08                     add      ax, 8
0054AB:  65 A3 46 67                  mov      word ptr gs:[0x6746], ax
0054AF:  65 89 1E 48 67               mov      word ptr gs:[0x6748], bx
0054B4:  EB 6E                        jmp      0x5524
0054B6:  BF C4 67                     mov      di, 0x67c4
0054B9:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
0054BE:  73 09                        jae      0x54c9
0054C0:  8B 44 10                     mov      ax, word ptr [si + 0x10]
0054C3:  65 A3 50 67                  mov      word ptr gs:[0x6750], ax
0054C7:  EB 5B                        jmp      0x5524
0054C9:  BF C9 67                     mov      di, 0x67c9
0054CC:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
0054D1:  73 09                        jae      0x54dc
0054D3:  8B 44 10                     mov      ax, word ptr [si + 0x10]
0054D6:  65 A3 54 67                  mov      word ptr gs:[0x6754], ax
0054DA:  EB 48                        jmp      0x5524
0054DC:  BF CE 67                     mov      di, 0x67ce
0054DF:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
0054E4:  73 09                        jae      0x54ef
0054E6:  8B 44 10                     mov      ax, word ptr [si + 0x10]
0054E9:  65 A3 56 67                  mov      word ptr gs:[0x6756], ax
0054ED:  EB 35                        jmp      0x5524
0054EF:  BF D3 67                     mov      di, 0x67d3
0054F2:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
0054F7:  73 09                        jae      0x5502
0054F9:  8B 44 10                     mov      ax, word ptr [si + 0x10]
0054FC:  65 A3 52 67                  mov      word ptr gs:[0x6752], ax
005500:  EB 22                        jmp      0x5524
005502:  BF E1 67                     mov      di, 0x67e1
005505:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
00550A:  73 07                        jae      0x5513
00550C:  8B 44 10                     mov      ax, word ptr [si + 0x10]
00550F:  65 A3 58 67                  mov      word ptr gs:[0x6758], ax
005513:  BF E5 67                     mov      di, 0x67e5
005516:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
00551B:  73 07                        jae      0x5524
00551D:  8B 44 10                     mov      ax, word ptr [si + 0x10]
005520:  65 A3 60 67                  mov      word ptr gs:[0x6760], ax
005524:  83 C6 14                     add      si, 0x14
005527:  E9 5C FF                     jmp      0x5486
00552A:  8B 44 12                     mov      ax, word ptr [si + 0x12]
00552D:  0B C0                        or       ax, ax
00552F:  74 1B                        je       0x554c
005531:  83 F8 05                     cmp      ax, 5
005534:  75 11                        jne      0x5547
005536:  BF F0 67                     mov      di, 0x67f0
005539:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
00553E:  73 07                        jae      0x5547
005540:  8B 44 10                     mov      ax, word ptr [si + 0x10]
005543:  65 A3 9C 67                  mov      word ptr gs:[0x679c], ax
005547:  83 C6 14                     add      si, 0x14
00554A:  EB DE                        jmp      0x552a
00554C:  33 C0                        xor      ax, ax
00554E:  EB 03                        jmp      0x5553
005550:  B8 FF FF                     mov      ax, 0xffff
005553:  5E                           pop      si
005554:  1F                           pop      ds
005555:  5F                           pop      di
005556:  07                           pop      es
005557:  5A                           pop      dx
005558:  59                           pop      cx
005559:  5B                           pop      bx
00555A:  CB                           retf    
