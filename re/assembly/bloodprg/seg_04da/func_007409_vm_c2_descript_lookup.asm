; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007409
; seg_off: 04da:2069
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vm_c2_descript_lookup
; label_comment: C2 kind-0x0400 helper: opens descript.des and dispatches matching descriptor script
; incoming: call@0x00189d->04da:2069
; incoming: call@0x007b2e->04da:2069
; incoming: call@0x00b116->04da:2069
; incoming: call@0x00b22d->04da:2069
; incoming: call@0x00b3bc->04da:2069
; byte_count: 277
; boundary: cfg_blocks_15_terminals_3
; terminal: jmp 0x74f8:1, jmp 0x7514:1, retf:1
; direct_callees: none
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_04da/func_007409_vm_c2_descript_lookup.cpp
; routine_bytes_sha256: 5e1f22afd92e9fe529c2d92df7b557c9e07381b64d1d74f413048fdd14cf0376

007409:  1E                           push     ds
00740A:  52                           push     dx
00740B:  06                           push     es
00740C:  57                           push     di
00740D:  53                           push     bx
00740E:  51                           push     cx
00740F:  52                           push     dx
007410:  55                           push     bp
007411:  56                           push     si
007412:  66 33 C0                     xor      eax, eax
007415:  8C E8                        mov      ax, gs
007417:  8E D8                        mov      ds, ax
007419:  C6 06 E8 27 00               mov      byte ptr [0x27e8], 0
00741E:  C6 06 A1 0B 00               mov      byte ptr [0xba1], 0
007423:  C6 06 1E 13 00               mov      byte ptr [0x131e], 0
007428:  BB 54 21                     mov      bx, 0x2154
00742B:  89 1E AD 1F                  mov      word ptr [0x1fad], bx
00742F:  33 DB                        xor      bx, bx
007431:  8A 1E DE 6C                  mov      bl, byte ptr [0x6cde]
007435:  C1 E3 07                     shl      bx, 7
007438:  81 C3 20 13                  add      bx, 0x1320
00743C:  89 1E 1A 13                  mov      word ptr [0x131a], bx
007440:  BB 1A 0F                     mov      bx, 0xf1a
007443:  89 1E 18 0F                  mov      word ptr [0xf18], bx
007447:  BB B5 1F                     mov      bx, 0x1fb5
00744A:  83 C3 26                     add      bx, 0x26
00744D:  89 1E AF 1F                  mov      word ptr [0x1faf], bx
007451:  C6 06 16 0B 00               mov      byte ptr [0xb16], 0
007456:  BA 06 01                     mov      dx, 0x106
007459:  9A B3 03 CE 01               lcall    0x1ce, 0x3b3
00745E:  B8 00 3D                     mov      ax, 0x3d00
007461:  CD 21                        int      0x21
007463:  0F 82 AB 00                  jb       0x7512
007467:  8B D8                        mov      bx, ax
007469:  B8 00 3F                     mov      ax, 0x3f00
00746C:  B9 02 00                     mov      cx, 2
00746F:  BA AE 0A                     mov      dx, 0xaae
007472:  CD 21                        int      0x21
007474:  B9 12 00                     mov      cx, 0x12
007477:  A1 AE 0A                     mov      ax, word ptr [0xaae]
00747A:  F7 E1                        mul      cx
00747C:  8B C8                        mov      cx, ax
00747E:  B8 00 3F                     mov      ax, 0x3f00
007481:  C5 16 BC 0A                  lds      dx, ptr [0xabc]
007485:  8B F2                        mov      si, dx
007487:  CD 21                        int      0x21
007489:  57                           push     di
00748A:  56                           push     si
00748B:  AC                           lodsb    al, byte ptr [si]
00748C:  47                           inc      di
00748D:  0A C0                        or       al, al
00748F:  75 07                        jne      0x7498
007491:  26 80 7D FF 00               cmp      byte ptr es:[di - 1], 0
007496:  74 16                        je       0x74ae
007498:  26 3A 45 FF                  cmp      al, byte ptr es:[di - 1]
00749C:  74 ED                        je       0x748b
00749E:  5E                           pop      si
00749F:  5F                           pop      di
0074A0:  83 C6 12                     add      si, 0x12
0074A3:  65 FF 0E AE 0A               dec      word ptr gs:[0xaae]
0074A8:  75 DF                        jne      0x7489
0074AA:  33 C0                        xor      ax, ax
0074AC:  EB 4A                        jmp      0x74f8
0074AE:  5E                           pop      si
0074AF:  5F                           pop      di
0074B0:  B8 00 42                     mov      ax, 0x4200
0074B3:  33 C9                        xor      cx, cx
0074B5:  8B 54 10                     mov      dx, word ptr [si + 0x10]
0074B8:  CD 21                        int      0x21
0074BA:  B9 02 00                     mov      cx, 2
0074BD:  8C E8                        mov      ax, gs
0074BF:  8E D8                        mov      ds, ax
0074C1:  8E C0                        mov      es, ax
0074C3:  B8 00 3F                     mov      ax, 0x3f00
0074C6:  BA B0 0A                     mov      dx, 0xab0
0074C9:  CD 21                        int      0x21
0074CB:  8B 0E B0 0A                  mov      cx, word ptr [0xab0]
0074CF:  83 E9 02                     sub      cx, 2
0074D2:  C5 16 BC 0A                  lds      dx, ptr [0xabc]
0074D6:  8B F2                        mov      si, dx
0074D8:  B8 00 3F                     mov      ax, 0x3f00
0074DB:  CD 21                        int      0x21
0074DD:  AC                           lodsb    al, byte ptr [si]
0074DE:  FE C8                        dec      al
0074E0:  98                           cwde    
0074E1:  78 12                        js       0x74f5
0074E3:  03 C0                        add      ax, ax
0074E5:  67 2E FF 90 7E 21 00 00      call     word ptr cs:[eax + 0x217e]
0074ED:  65 F6 06 16 0B 01            test     byte ptr gs:[0xb16], 1
0074F3:  74 E8                        je       0x74dd
0074F5:  B8 01 00                     mov      ax, 1
0074F8:  50                           push     ax
0074F9:  65 8B 2E 18 0F               mov      bp, word ptr gs:[0xf18]
0074FE:  C7 46 00 FF FF               mov      word ptr [bp], 0xffff
007503:  B8 1A 0F                     mov      ax, 0xf1a
007506:  65 A3 18 0F                  mov      word ptr gs:[0xf18], ax
00750A:  B8 00 3E                     mov      ax, 0x3e00
00750D:  CD 21                        int      0x21
00750F:  58                           pop      ax
007510:  EB 02                        jmp      0x7514
007512:  33 C0                        xor      ax, ax
007514:  5E                           pop      si
007515:  5D                           pop      bp
007516:  5A                           pop      dx
007517:  59                           pop      cx
007518:  5B                           pop      bx
007519:  5F                           pop      di
00751A:  07                           pop      es
00751B:  5A                           pop      dx
00751C:  1F                           pop      ds
00751D:  CB                           retf    
