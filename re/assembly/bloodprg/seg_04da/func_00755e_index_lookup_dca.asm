; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00755e
; seg_off: 04da:21be
; group: seg_04da
; provenance: static_dispatch_table_target
; label: index_lookup_dca
; label_comment: index-table lookup: di=0xdca; lodsb al (operand); dec al; cbw (index-1 signed). Indexes a table at DS:0xdca by the script operand
; incoming: byte_parser_dispatch_74e5:byte_0x03
; byte_count: 180
; boundary: cfg_blocks_16_terminals_3
; terminal: jmp 0x756b:1, jmp 0x758f:1, ret:1
; direct_callees: none
; indirect_calls: 3
; cxx_source: re/borland/bloodprg/seg_04da/func_00755e_index_lookup_dca.cpp
; routine_bytes_sha256: 3a600e6ae82ab1392ea69afdee2ce9fde0f3d65e1017f91b416a04cfbcf28d63

00755E:  53                           push     bx
00755F:  BF CA 0D                     mov      di, 0xdca
007562:  AC                           lodsb    al, byte ptr [si]
007563:  FE C8                        dec      al
007565:  98                           cwde    
007566:  C1 E0 04                     shl      ax, 4
007569:  8B D8                        mov      bx, ax
00756B:  AC                           lodsb    al, byte ptr [si]
00756C:  0A C0                        or       al, al
00756E:  78 07                        js       0x7577
007570:  3C 20                        cmp      al, 0x20
007572:  72 03                        jb       0x7577
007574:  AA                           stosb    byte ptr es:[di], al
007575:  EB F4                        jmp      0x756b
007577:  4E                           dec      si
007578:  26 C6 05 00                  mov      byte ptr es:[di], 0
00757C:  1E                           push     ds
00757D:  56                           push     si
00757E:  8C E8                        mov      ax, gs
007580:  8E D8                        mov      ds, ax
007582:  66 33 D2                     xor      edx, edx
007585:  BA D7 0D                     mov      dx, 0xdd7
007588:  03 D3                        add      dx, bx
00758A:  8B FA                        mov      di, dx
00758C:  BE CA 0D                     mov      si, 0xdca
00758F:  AC                           lodsb    al, byte ptr [si]
007590:  0A C0                        or       al, al
007592:  74 7A                        je       0x760e
007594:  26 3A 05                     cmp      al, byte ptr es:[di]
007597:  75 03                        jne      0x759c
007599:  47                           inc      di
00759A:  EB F3                        jmp      0x758f
00759C:  8B FA                        mov      di, dx
00759E:  BE CA 0D                     mov      si, 0xdca
0075A1:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
0075A6:  B8 00 41                     mov      ax, 0x4100
0075A9:  CD 21                        int      0x21
0075AB:  AC                           lodsb    al, byte ptr [si]
0075AC:  AA                           stosb    byte ptr es:[di], al
0075AD:  0A C0                        or       al, al
0075AF:  75 FA                        jne      0x75ab
0075B1:  33 C9                        xor      cx, cx
0075B3:  B8 00 3C                     mov      ax, 0x3c00
0075B6:  CD 21                        int      0x21
0075B8:  50                           push     ax
0075B9:  BA C7 0D                     mov      dx, 0xdc7
0075BC:  9A B3 03 CE 01               lcall    0x1ce, 0x3b3
0075C1:  F6 06 E2 0A 01               test     byte ptr [0xae2], 1
0075C6:  75 16                        jne      0x75de
0075C8:  8B F2                        mov      si, dx
0075CA:  9A EA 05 CE 01               lcall    0x1ce, 0x5ea
0075CF:  66 89 2E 92 0A               mov      dword ptr [0xa92], ebp
0075D4:  66 33 ED                     xor      ebp, ebp
0075D7:  B8 00 3D                     mov      ax, 0x3d00
0075DA:  CD 21                        int      0x21
0075DC:  8B D8                        mov      bx, ax
0075DE:  C5 16 29 52                  lds      dx, ptr [0x5229]
0075E2:  5F                           pop      di
0075E3:  65 8B 0E 92 0A               mov      cx, word ptr gs:[0xa92]
0075E8:  B8 00 3F                     mov      ax, 0x3f00
0075EB:  CD 21                        int      0x21
0075ED:  8B C8                        mov      cx, ax
0075EF:  87 DF                        xchg     di, bx
0075F1:  B4 40                        mov      ah, 0x40
0075F3:  CD 21                        int      0x21
0075F5:  87 DF                        xchg     di, bx
0075F7:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
0075FD:  75 05                        jne      0x7604
0075FF:  B8 00 3E                     mov      ax, 0x3e00
007602:  CD 21                        int      0x21
007604:  8B DF                        mov      bx, di
007606:  B8 00 3E                     mov      ax, 0x3e00
007609:  CD 21                        int      0x21
00760B:  66 33 C0                     xor      eax, eax
00760E:  5E                           pop      si
00760F:  1F                           pop      ds
007610:  5B                           pop      bx
007611:  C3                           ret     
