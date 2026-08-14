; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00739b
; seg_off: 04da:1ffb
; group: seg_04da
; provenance: recursive_graph
; label: vm_cod_scan
; label_comment: marks A6 text tokens for one object in the top-level script and all A6 tokens in that object's code block; preserves the full query-mode word while using vm_token_advance
; byte_count: 110
; boundary: cfg_blocks_14_terminals_3
; terminal: jmp 0x73aa:1, jmp 0x73e7:1, ret:1
; direct_callees: 0x006023, 0x0062b6
; indirect_calls: 0
; routine_bytes_sha256: 378ed47245448b5c6573030e6e2128dd0a22c3ce1d73c1ac36ebd24239b55973

00739B:  1E                           push     ds
00739C:  56                           push     si
00739D:  06                           push     es
00739E:  57                           push     di
00739F:  50                           push     ax
0073A0:  65 FF 36 AD 67               push     word ptr gs:[0x67ad]
0073A5:  65 C5 36 1C 67               lds      si, ptr gs:[0x671c]
0073AA:  AC                           lodsb    al, byte ptr [si]
0073AB:  3C FF                        cmp      al, 0xff
0073AD:  74 12                        je       0x73c1
0073AF:  3C A6                        cmp      al, 0xa6
0073B1:  75 08                        jne      0x73bb
0073B3:  3B 1C                        cmp      bx, word ptr [si]
0073B5:  75 04                        jne      0x73bb
0073B7:  80 4C 04 80                  or       byte ptr [si + 4], 0x80
0073BB:  4E                           dec      si
0073BC:  E8 F7 EE                     call     0x62b6
0073BF:  EB E9                        jmp      0x73aa
0073C1:  65 C6 06 B2 67 01            mov      byte ptr gs:[0x67b2], 1
0073C7:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
0073CC:  65 C5 36 20 67               lds      si, ptr gs:[0x6720]
0073D1:  03 FB                        add      di, bx
0073D3:  26 8B 1D                     mov      bx, word ptr es:[di]
0073D6:  B8 02 00                     mov      ax, 2
0073D9:  E8 47 EC                     call     0x6023
0073DC:  03 F8                        add      di, ax
0073DE:  26 8B 05                     mov      ax, word ptr es:[di]
0073E1:  0B C0                        or       ax, ax
0073E3:  74 19                        je       0x73fe
0073E5:  03 F0                        add      si, ax
0073E7:  AC                           lodsb    al, byte ptr [si]
0073E8:  3C FF                        cmp      al, 0xff
0073EA:  74 12                        je       0x73fe
0073EC:  3C AA                        cmp      al, 0xaa
0073EE:  74 0E                        je       0x73fe
0073F0:  3C A6                        cmp      al, 0xa6
0073F2:  75 04                        jne      0x73f8
0073F4:  80 4C 04 80                  or       byte ptr [si + 4], 0x80
0073F8:  4E                           dec      si
0073F9:  E8 BA EE                     call     0x62b6
0073FC:  EB E9                        jmp      0x73e7
0073FE:  65 8F 06 AD 67               pop      word ptr gs:[0x67ad]
007403:  58                           pop      ax
007404:  5F                           pop      di
007405:  07                           pop      es
007406:  5E                           pop      si
007407:  1F                           pop      ds
007408:  C3                           ret     
