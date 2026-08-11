; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0092a3
; seg_off: 071e:1ac3
; group: seg_071e
; provenance: recursive_graph
; label: nav_chart_object_pick
; label_comment: NAV-CHART OBJECT PICKER: walks the visible-object list at DS:0x2AD3 ([0x27C1] entries) and hit-tests each marker. Marker position = the object's selector-0x0B field (FIELD_OFFSETS[0x0B] = 0x18 for kinds 8 and 0x10): x at +0x18, y at +0x1A. Box size BY KIND: default (0x0C,0x0B); kind&0x100 BLACK HOLE (0x13,0x0C); kind&0x10 SHIP (0x15,0x0A). A black hole has TWO chart positions -- +0x18/+0x1A and +0x1C/+0x1E -- and uses the second when es:[obj+0x14] != es:[arche+0x22] (0x92DF..0x92F2), i.e. the two ends of the same wormhole. Box origin is (x-2,y-2) and both bounds are INCLUSIVE (jb/ja). Returns the FIRST hit in list order, else 0. Caller 0x8FB0: with the mouse ENABLED the hit becomes the info-panel selection (0x8FF4), with it disabled the object's name is drawn as a hover label. PORTED: vm.rs nav_chart_pick
; byte_count: 151
; boundary: cfg_blocks_13_terminals_3
; terminal: jmp 0x9308:1, jmp 0x9339:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 09e896293459b3d194e3d736028340d9ad032d641d356822be7f3ae36cb60d24

0092A3:  33 C0                        xor      ax, ax
0092A5:  8B 0E C1 27                  mov      cx, word ptr [0x27c1]
0092A9:  0B C9                        or       cx, cx
0092AB:  0F 84 8A 00                  je       0x9339
0092AF:  BD D3 2A                     mov      bp, 0x2ad3
0092B2:  A1 2A 0A                     mov      ax, word ptr [0xa2a]
0092B5:  8B 16 2C 0A                  mov      dx, word ptr [0xa2c]
0092B9:  8B 7E 00                     mov      di, word ptr [bp]
0092BC:  83 C7 18                     add      di, 0x18
0092BF:  C7 06 7A 27 0C 00            mov      word ptr [0x277a], 0xc
0092C5:  C7 06 7C 27 0B 00            mov      word ptr [0x277c], 0xb
0092CB:  26 F7 45 E8 00 01            test     word ptr es:[di - 0x18], 0x100
0092D1:  74 21                        je       0x92f4
0092D3:  C7 06 7A 27 13 00            mov      word ptr [0x277a], 0x13
0092D9:  C7 06 7C 27 0C 00            mov      word ptr [0x277c], 0xc
0092DF:  8B 1E 52 67                  mov      bx, word ptr [0x6752]
0092E3:  83 C3 22                     add      bx, 0x22
0092E6:  26 8B 1F                     mov      bx, word ptr es:[bx]
0092E9:  26 3B 5D FC                  cmp      bx, word ptr es:[di - 4]
0092ED:  74 05                        je       0x92f4
0092EF:  83 C7 04                     add      di, 4
0092F2:  EB 14                        jmp      0x9308
0092F4:  26 F7 45 E8 10 00            test     word ptr es:[di - 0x18], 0x10
0092FA:  74 0C                        je       0x9308
0092FC:  C7 06 7A 27 15 00            mov      word ptr [0x277a], 0x15
009302:  C7 06 7C 27 0A 00            mov      word ptr [0x277c], 0xa
009308:  26 8B 1D                     mov      bx, word ptr es:[di]
00930B:  83 EB 02                     sub      bx, 2
00930E:  3B C3                        cmp      ax, bx
009310:  72 20                        jb       0x9332
009312:  03 1E 7A 27                  add      bx, word ptr [0x277a]
009316:  3B C3                        cmp      ax, bx
009318:  77 18                        ja       0x9332
00931A:  26 8B 5D 02                  mov      bx, word ptr es:[di + 2]
00931E:  83 EB 02                     sub      bx, 2
009321:  3B D3                        cmp      dx, bx
009323:  72 0D                        jb       0x9332
009325:  03 1E 7C 27                  add      bx, word ptr [0x277c]
009329:  3B D3                        cmp      dx, bx
00932B:  77 05                        ja       0x9332
00932D:  8B 46 00                     mov      ax, word ptr [bp]
009330:  EB 07                        jmp      0x9339
009332:  83 C5 02                     add      bp, 2
009335:  E2 82                        loop     0x92b9
009337:  33 C0                        xor      ax, ax
009339:  C3                           ret     
