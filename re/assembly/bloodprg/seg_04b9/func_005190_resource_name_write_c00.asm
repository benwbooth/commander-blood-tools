; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005190
; seg_off: 04b9:0000
; group: seg_04b9
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_allocate
; label_comment: Complete resource allocator. AX selects the handle, EBP supplies byte count, and the routine reuses or allocates storage with reverse lock-aware eviction before returning status in AX and destination in DS:SI.
; incoming: call@0x000eba->04b9:0000
; incoming: call@0x000ecd->04b9:0000
; incoming: call@0x000ee1->04b9:0000
; incoming: call@0x000efb->04b9:0000
; incoming: call@0x000f12->04b9:0000
; incoming: call@0x000f24->04b9:0000
; incoming: call@0x00289b->04b9:0000
; incoming: call@0x00402b->04b9:0000
; byte_count: 248
; boundary: cfg_blocks_15_terminals_4
; terminal: jmp 0x51b3:1, jmp 0x521e:1, jmp 0x527d:1, retf:1
; direct_callees: 0x00529c
; indirect_calls: 1
; routine_bytes_sha256: 8a3cab21e7ae6a9c952f5a78826d986db329184368f10b612abe98b79fe14d24

005190:  66 53                        push     ebx
005192:  66 51                        push     ecx
005194:  66 52                        push     edx
005196:  66 55                        push     ebp
005198:  06                           push     es
005199:  57                           push     di
00519A:  8C E3                        mov      bx, fs
00519C:  8E DB                        mov      ds, bx
00519E:  8E C3                        mov      es, bx
0051A0:  A3 00 0C                     mov      word ptr [0xc00], ax
0051A3:  8B D8                        mov      bx, ax
0051A5:  C1 E3 03                     shl      bx, 3
0051A8:  89 1E 02 0C                  mov      word ptr [0xc02], bx
0051AC:  F7 47 02 03 00               test     word ptr [bx + 2], 3
0051B1:  74 10                        je       0x51c3
0051B3:  83 4F 02 02                  or       word ptr [bx + 2], 2
0051B7:  8B 07                        mov      ax, word ptr [bx]
0051B9:  8E D8                        mov      ds, ax
0051BB:  33 F6                        xor      si, si
0051BD:  B8 01 00                     mov      ax, 1
0051C0:  E9 BA 00                     jmp      0x527d
0051C3:  66 83 C5 0F                  add      ebp, 0xf
0051C7:  66 83 E5 F0                  and      ebp, 0xfffffff0
0051CB:  66 65 3B 2E 46 0A            cmp      ebp, dword ptr gs:[0xa46]
0051D1:  7E 56                        jle      0x5229
0051D3:  BF 00 08                     mov      di, 0x800
0051D6:  B9 00 01                     mov      cx, 0x100
0051D9:  B8 FF FF                     mov      ax, 0xffff
0051DC:  F2 AF                        repne scasw ax, word ptr es:[di]
0051DE:  66 8B CD                     mov      ecx, ebp
0051E1:  66 65 2B 0E 46 0A            sub      ecx, dword ptr gs:[0xa46]
0051E7:  83 EF 02                     sub      di, 2
0051EA:  8B F7                        mov      si, di
0051EC:  83 EE 02                     sub      si, 2
0051EF:  BF 00 0A                     mov      di, 0xa00
0051F2:  FD                           std     
0051F3:  AD                           lodsw    ax, word ptr [si]
0051F4:  8B D8                        mov      bx, ax
0051F6:  C1 E3 03                     shl      bx, 3
0051F9:  F7 47 02 02 00               test     word ptr [bx + 2], 2
0051FE:  75 0B                        jne      0x520b
005200:  89 05                        mov      word ptr [di], ax
005202:  83 C7 02                     add      di, 2
005205:  66 2B 4F 04                  sub      ecx, dword ptr [bx + 4]
005209:  7E 06                        jle      0x5211
00520B:  81 FE 00 08                  cmp      si, 0x800
00520F:  7D E2                        jge      0x51f3
005211:  FC                           cld     
005212:  B8 FF FF                     mov      ax, 0xffff
005215:  AB                           stosw    word ptr es:[di], ax
005216:  66 0B C9                     or       ecx, ecx
005219:  79 57                        jns      0x5272
00521B:  BE 00 0A                     mov      si, 0xa00
00521E:  AD                           lodsw    ax, word ptr [si]
00521F:  0B C0                        or       ax, ax
005221:  78 06                        js       0x5229
005223:  0E                           push     cs
005224:  E8 75 00                     call     0x529c
005227:  EB F5                        jmp      0x521e
005229:  8B 1E 02 0C                  mov      bx, word ptr [0xc02]
00522D:  65 A1 6A 0A                  mov      ax, word ptr gs:[0xa6a]
005231:  89 07                        mov      word ptr [bx], ax
005233:  83 4F 02 03                  or       word ptr [bx + 2], 3
005237:  66 89 6F 04                  mov      dword ptr [bx + 4], ebp
00523B:  66 65 29 2E 46 0A            sub      dword ptr gs:[0xa46], ebp
005241:  66 C1 ED 04                  shr      ebp, 4
005245:  65 01 2E 6A 0A               add      word ptr gs:[0xa6a], bp
00524A:  BF 00 08                     mov      di, 0x800
00524D:  B8 FF FF                     mov      ax, 0xffff
005250:  B9 00 01                     mov      cx, 0x100
005253:  F2 AF                        repne scasw ax, word ptr es:[di]
005255:  A1 00 0C                     mov      ax, word ptr [0xc00]
005258:  89 45 FE                     mov      word ptr [di - 2], ax
00525B:  C7 05 FF FF                  mov      word ptr [di], 0xffff
00525F:  8B 07                        mov      ax, word ptr [bx]
005261:  8E D8                        mov      ds, ax
005263:  33 F6                        xor      si, si
005265:  33 C0                        xor      ax, ax
005267:  26 F7 47 02 0C 00            test     word ptr es:[bx + 2], 0xc
00526D:  74 0E                        je       0x527d
00526F:  E9 41 FF                     jmp      0x51b3
005272:  B8 02 00                     mov      ax, 2
005275:  9A 75 07 00 00               lcall    0, 0x775
00527A:  B8 FF FF                     mov      ax, 0xffff
00527D:  5F                           pop      di
00527E:  07                           pop      es
00527F:  66 5D                        pop      ebp
005281:  66 5A                        pop      edx
005283:  66 59                        pop      ecx
005285:  66 5B                        pop      ebx
005287:  CB                           retf    
