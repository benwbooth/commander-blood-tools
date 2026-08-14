; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0062b6
; seg_off: 04da:0f16
; group: seg_04da
; provenance: recursive_graph
; label: vm_token_advance
; label_comment: decode+skip one DS:SI script token; biases op by 0xa0 and uses the descriptor table through BP at SS:0x6f18 (runtime SS=GS), leaving DS on the script image
; byte_count: 131
; boundary: cfg_blocks_21_terminals_6
; terminal: jmp 0x6316:4, jmp 0x6335:1, ret:1
; direct_callees: 0x006293
; indirect_calls: 0
; routine_bytes_sha256: 842c4deffde9b5b7b2a580569f372312d12238562730ca49ce23f5d062ea68d2

0062B6:  50                           push     ax
0062B7:  53                           push     bx
0062B8:  55                           push     bp
0062B9:  BD 18 6F                     mov      bp, 0x6f18
0062BC:  AC                           lodsb    al, byte ptr [si]
0062BD:  8A D8                        mov      bl, al
0062BF:  2C A0                        sub      al, 0xa0
0062C1:  98                           cwde    
0062C2:  03 C0                        add      ax, ax
0062C4:  03 E8                        add      bp, ax
0062C6:  8A 46 01                     mov      al, byte ptr [bp + 1]
0062C9:  0A C0                        or       al, al
0062CB:  78 0C                        js       0x62d9
0062CD:  65 A0 AD 67                  mov      al, byte ptr gs:[0x67ad]
0062D1:  98                           cwde    
0062D2:  03 E8                        add      bp, ax
0062D4:  8A 46 00                     mov      al, byte ptr [bp]
0062D7:  EB 3D                        jmp      0x6316
0062D9:  3C FF                        cmp      al, 0xff
0062DB:  75 0B                        jne      0x62e8
0062DD:  65 C6 06 AD 67 01            mov      byte ptr gs:[0x67ad], 1
0062E3:  8A 46 00                     mov      al, byte ptr [bp]
0062E6:  EB 2E                        jmp      0x6316
0062E8:  3C FE                        cmp      al, 0xfe
0062EA:  75 0B                        jne      0x62f7
0062EC:  65 C6 06 AD 67 00            mov      byte ptr gs:[0x67ad], 0
0062F2:  8A 46 00                     mov      al, byte ptr [bp]
0062F5:  EB 1F                        jmp      0x6316
0062F7:  3C FD                        cmp      al, 0xfd
0062F9:  75 0C                        jne      0x6307
0062FB:  8A 04                        mov      al, byte ptr [si]
0062FD:  3C A1                        cmp      al, 0xa1
0062FF:  75 01                        jne      0x6302
006301:  46                           inc      si
006302:  8A 46 00                     mov      al, byte ptr [bp]
006305:  EB 0F                        jmp      0x6316
006307:  65 F6 06 B2 67 01            test     byte ptr gs:[0x67b2], 1
00630D:  75 0B                        jne      0x631a
00630F:  3C FB                        cmp      al, 0xfb
006311:  74 E8                        je       0x62fb
006313:  8A 46 00                     mov      al, byte ptr [bp]
006316:  0A C0                        or       al, al
006318:  75 16                        jne      0x6330
00631A:  80 FB A6                     cmp      bl, 0xa6
00631D:  75 0A                        jne      0x6329
00631F:  83 C6 05                     add      si, 5
006322:  AD                           lodsw    ax, word ptr [si]
006323:  0B C0                        or       ax, ax
006325:  75 FB                        jne      0x6322
006327:  EB 0C                        jmp      0x6335
006329:  33 C0                        xor      ax, ax
00632B:  E8 65 FF                     call     0x6293
00632E:  B0 01                        mov      al, 1
006330:  FE C8                        dec      al
006332:  98                           cwde    
006333:  03 F0                        add      si, ax
006335:  5D                           pop      bp
006336:  5B                           pop      bx
006337:  58                           pop      ax
006338:  C3                           ret     
