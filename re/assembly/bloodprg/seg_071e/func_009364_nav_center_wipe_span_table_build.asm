; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009364
; seg_off: 071e:1b84
; group: seg_071e
; provenance: recursive_graph
; label: nav_center_wipe_span_table_build
; label_comment: NAVIGATION CENTER-WIPE SPAN-TABLE BUILDER: reads one signed (x,y) endpoint from DS:SI, orders it with center (160,110) by Y, and runs 16-bit Bresenham error arithmetic. For every affected scanline it writes (left=x, width=2*(160-x)) as two words through the full far pointer at DS:0x5221, then appends 0xFFFF,0xFFFF. It builds data consumed by caller 0x8CCE's row-copy loops; it does not draw pixels itself. The shipped endpoint table at DS:0x2752 is (160,0),(140,0),(120,0),(60,0),(0,0),(0,50),(0,90),(0,130),(0,190). Equal deltas select the vertical-major path; a center endpoint produces 65536 spans because LOOP starts at zero. NATURAL C: func_009364_nav_center_wipe_span_table_build.c
; byte_count: 145
; boundary: cfg_blocks_12_terminals_2
; terminal: jmp 0x93e7:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ff5a7011935b6206a9c3bd69d939108d3e2f36613bd0160236a3af05584c3dad

009364:  50                           push     ax
009365:  53                           push     bx
009366:  51                           push     cx
009367:  52                           push     dx
009368:  55                           push     bp
009369:  06                           push     es
00936A:  57                           push     di
00936B:  56                           push     si
00936C:  C4 3E 21 52                  les      di, ptr [0x5221]
009370:  BA A0 00                     mov      dx, 0xa0
009373:  BD 6E 00                     mov      bp, 0x6e
009376:  AD                           lodsw    ax, word ptr [si]
009377:  8B D8                        mov      bx, ax
009379:  AD                           lodsw    ax, word ptr [si]
00937A:  8B C8                        mov      cx, ax
00937C:  3B CD                        cmp      cx, bp
00937E:  7C 04                        jl       0x9384
009380:  87 DA                        xchg     dx, bx
009382:  87 CD                        xchg     bp, cx
009384:  8B C3                        mov      ax, bx
009386:  2B D3                        sub      dx, bx
009388:  2B E9                        sub      bp, cx
00938A:  BE 01 00                     mov      si, 1
00938D:  0B D2                        or       dx, dx
00938F:  79 04                        jns      0x9395
009391:  F7 DA                        neg      dx
009393:  F7 DE                        neg      si
009395:  3B EA                        cmp      bp, dx
009397:  7C 29                        jl       0x93c2
009399:  87 EA                        xchg     dx, bp
00939B:  8B DD                        mov      bx, bp
00939D:  03 DB                        add      bx, bx
00939F:  2B DA                        sub      bx, dx
0093A1:  8B CA                        mov      cx, dx
0093A3:  03 ED                        add      bp, bp
0093A5:  03 D2                        add      dx, dx
0093A7:  0B DB                        or       bx, bx
0093A9:  AB                           stosw    word ptr es:[di], ax
0093AA:  9C                           pushf   
0093AB:  50                           push     ax
0093AC:  2D A0 00                     sub      ax, 0xa0
0093AF:  F7 D8                        neg      ax
0093B1:  03 C0                        add      ax, ax
0093B3:  AB                           stosw    word ptr es:[di], ax
0093B4:  58                           pop      ax
0093B5:  9D                           popf    
0093B6:  78 04                        js       0x93bc
0093B8:  03 C6                        add      ax, si
0093BA:  2B DA                        sub      bx, dx
0093BC:  03 DD                        add      bx, bp
0093BE:  E2 E9                        loop     0x93a9
0093C0:  EB 25                        jmp      0x93e7
0093C2:  8B DD                        mov      bx, bp
0093C4:  03 DB                        add      bx, bx
0093C6:  2B DA                        sub      bx, dx
0093C8:  8B CA                        mov      cx, dx
0093CA:  03 ED                        add      bp, bp
0093CC:  03 D2                        add      dx, dx
0093CE:  0B DB                        or       bx, bx
0093D0:  9C                           pushf   
0093D1:  03 C6                        add      ax, si
0093D3:  9D                           popf    
0093D4:  78 0D                        js       0x93e3
0093D6:  50                           push     ax
0093D7:  AB                           stosw    word ptr es:[di], ax
0093D8:  2D A0 00                     sub      ax, 0xa0
0093DB:  F7 D8                        neg      ax
0093DD:  03 C0                        add      ax, ax
0093DF:  AB                           stosw    word ptr es:[di], ax
0093E0:  58                           pop      ax
0093E1:  2B DA                        sub      bx, dx
0093E3:  03 DD                        add      bx, bp
0093E5:  E2 E9                        loop     0x93d0
0093E7:  B8 FF FF                     mov      ax, 0xffff
0093EA:  AB                           stosw    word ptr es:[di], ax
0093EB:  AB                           stosw    word ptr es:[di], ax
0093EC:  5E                           pop      si
0093ED:  5F                           pop      di
0093EE:  07                           pop      es
0093EF:  5D                           pop      bp
0093F0:  5A                           pop      dx
0093F1:  59                           pop      cx
0093F2:  5B                           pop      bx
0093F3:  58                           pop      ax
0093F4:  C3                           ret     
