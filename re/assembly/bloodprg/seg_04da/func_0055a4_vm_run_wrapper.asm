; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0055a4
; seg_off: 04da:0204
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vm_run_wrapper
; label_comment: prepares script resource far pointers, calls BIOS RTC writers, then enters vm_exec_loop || ALSO RECORDED as `vm_run_wrapper_is_the_per_frame_gameplay`: ARCHITECTURE: the main loop's per-frame update (lcall 0x4da:0x204, gated on [0x27e0]&1) IS vm_run_wrapper 0x55a4 - it resolves the 5 script-profile resources (DS:0x6712->DS:0x671c far ptrs via 0x55d9) then enters vm_exec_loop, gated on VM-active [0x67a8]&1. KEY: gameplay = VM SCRIPT EXECUTION. The object-simulation, combat, and interactions are script opcodes the VM runs each frame over the loaded profile (COD/BAS/VAR/DIC) - NOT a separate native loop. Unifies the decoded subsystems: main loop -> vm_run_wrapper -> vm_exec_loop -> opcodes (A6 text/C1 ship-3d/C4 presentation/D2 profile) -> entity table (0x6212) + render path. The entity accessors are called from VM opcode handlers || MERGED 2026-07-25 (audit-fixes #184): one address under several names, folded by union.
; incoming: call@0x001083->04da:0204
; incoming: call@0x0010de->04da:0204
; incoming: call@0x001d05->04da:0204
; byte_count: 258
; boundary: cfg_blocks_19_terminals_5
; terminal: jmp 0x5613:1, jmp 0x567b:2, jmp 0x569e:1, retf:1
; direct_callees: 0x005791, 0x005816, 0x005a74, 0x0062b6
; indirect_calls: 5
; routine_bytes_sha256: 5b1e8001ff77d92b99b51fd5bd530c42d3fe55c4b6077a81d6d20ca18efedb5e

0055A4:  53                           push     bx
0055A5:  51                           push     cx
0055A6:  52                           push     dx
0055A7:  1E                           push     ds
0055A8:  56                           push     si
0055A9:  06                           push     es
0055AA:  57                           push     di
0055AB:  33 C0                        xor      ax, ax
0055AD:  F6 06 A8 67 01               test     byte ptr [0x67a8], 1
0055B2:  0F 84 E8 00                  je       0x569e
0055B6:  9A 3B 03 00 00               lcall    0, 0x33b
0055BB:  9A 50 03 00 00               lcall    0, 0x350
0055C0:  66 33 C0                     xor      eax, eax
0055C3:  66 8B E8                     mov      ebp, eax
0055C6:  66 8B D8                     mov      ebx, eax
0055C9:  66 8B C8                     mov      ecx, eax
0055CC:  66 8B D0                     mov      edx, eax
0055CF:  66 8B F0                     mov      esi, eax
0055D2:  66 8B F8                     mov      edi, eax
0055D5:  8C E8                        mov      ax, gs
0055D7:  8E C0                        mov      es, ax
0055D9:  BF 1C 67                     mov      di, 0x671c
0055DC:  BD 12 67                     mov      bp, 0x6712
0055DF:  B9 05 00                     mov      cx, 5
0055E2:  8B 46 00                     mov      ax, word ptr [bp]
0055E5:  9A 90 01 B9 04               lcall    0x4b9, 0x190
0055EA:  8B C6                        mov      ax, si
0055EC:  AB                           stosw    word ptr es:[di], ax
0055ED:  8C D8                        mov      ax, ds
0055EF:  AB                           stosw    word ptr es:[di], ax
0055F0:  83 C5 02                     add      bp, 2
0055F3:  E2 ED                        loop     0x55e2
0055F5:  E8 7C 04                     call     0x5a74
0055F8:  65 C6 06 B2 67 00            mov      byte ptr gs:[0x67b2], 0
0055FE:  BF B0 6E                     mov      di, 0x6eb0
005601:  65 C5 36 1C 67               lds      si, ptr gs:[0x671c]
005606:  65 F6 06 B1 67 02            test     byte ptr gs:[0x67b1], 2
00560C:  74 05                        je       0x5613
00560E:  65 8B 36 7A 67               mov      si, word ptr gs:[0x677a]
005613:  AC                           lodsb    al, byte ptr [si]
005614:  3C FF                        cmp      al, 0xff
005616:  74 72                        je       0x568a
005618:  8A D8                        mov      bl, al
00561A:  80 EB A0                     sub      bl, 0xa0
00561D:  32 FF                        xor      bh, bh
00561F:  03 DB                        add      bx, bx
005621:  65 C6 06 B4 67 00            mov      byte ptr gs:[0x67b4], 0
005627:  65 FF 11                     call     word ptr gs:[bx + di]
00562A:  65 A0 B4 67                  mov      al, byte ptr gs:[0x67b4]
00562E:  0A C0                        or       al, al
005630:  75 29                        jne      0x565b
005632:  65 F6 06 AB 67 0F            test     byte ptr gs:[0x67ab], 0xf
005638:  74 0C                        je       0x5646
00563A:  E8 79 0C                     call     0x62b6
00563D:  65 FE 0E AB 67               dec      byte ptr gs:[0x67ab]
005642:  75 F6                        jne      0x563a
005644:  EB 35                        jmp      0x567b
005646:  65 80 3E B1 67 01            cmp      byte ptr gs:[0x67b1], 1
00564C:  75 2D                        jne      0x567b
00564E:  65 C6 06 B1 67 00            mov      byte ptr gs:[0x67b1], 0
005654:  65 8B 36 78 67               mov      si, word ptr gs:[0x6778]
005659:  EB B8                        jmp      0x5613
00565B:  65 C6 06 B7 67 01            mov      byte ptr gs:[0x67b7], 1
005661:  2C 02                        sub      al, 2
005663:  75 08                        jne      0x566d
005665:  65 C6 06 AB 67 00            mov      byte ptr gs:[0x67ab], 0
00566B:  EB 0E                        jmp      0x567b
00566D:  FE C8                        dec      al
00566F:  75 23                        jne      0x5694
005671:  65 FE 06 B1 67               inc      byte ptr gs:[0x67b1]
005676:  65 89 36 7A 67               mov      word ptr gs:[0x677a], si
00567B:  65 F6 06 B1 67 02            test     byte ptr gs:[0x67b1], 2
005681:  74 90                        je       0x5613
005683:  65 3B 36 78 67               cmp      si, word ptr gs:[0x6778]
005688:  72 89                        jb       0x5613
00568A:  E8 04 01                     call     0x5791
00568D:  E8 86 01                     call     0x5816
005690:  33 C0                        xor      ax, ax
005692:  EB 0A                        jmp      0x569e
005694:  33 C0                        xor      ax, ax
005696:  9A 75 07 00 00               lcall    0, 0x775
00569B:  B8 FF FF                     mov      ax, 0xffff
00569E:  5F                           pop      di
00569F:  07                           pop      es
0056A0:  5E                           pop      si
0056A1:  1F                           pop      ds
0056A2:  5A                           pop      dx
0056A3:  59                           pop      cx
0056A4:  5B                           pop      bx
0056A5:  CB                           retf    
