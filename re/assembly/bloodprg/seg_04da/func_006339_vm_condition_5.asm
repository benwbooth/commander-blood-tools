; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006339
; seg_off: 05d3:0009
; group: seg_04da
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: vm_condition_5
; label_comment: VM conditional helper for A6 text handling; consumes flags in CX, reads operands from DS:SI, and returns condition result in CF
; incoming: call@0x006680->0x006339
; byte_count: 250
; boundary: cfg_blocks_31_terminals_1
; terminal: ret:1
; direct_callees: 0x006293
; indirect_calls: 1
; routine_bytes_sha256: 579556b857f39be74532b019ebb3addc64e5fee5b3be37f3105b3bb1b162c3fe

006339:  56                           push     si
00633A:  55                           push     bp
00633B:  F6 C1 02                     test     cl, 2
00633E:  74 0E                        je       0x634e
006340:  B8 05 00                     mov      ax, 5
006343:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
006348:  0B C0                        or       ax, ax
00634A:  0F 85 E1 00                  jne      0x642f
00634E:  F6 C1 04                     test     cl, 4
006351:  74 35                        je       0x6388
006353:  8A C5                        mov      al, ch
006355:  D0 E8                        shr      al, 1
006357:  83 E0 07                     and      ax, 7
00635A:  40                           inc      ax
00635B:  C1 E0 04                     shl      ax, 4
00635E:  40                           inc      ax
00635F:  93                           xchg     bx, ax
006360:  65 8A 9F 60 6D               mov      bl, byte ptr gs:[bx + 0x6d60]
006365:  26 8B 11                     mov      dx, word ptr es:[bx + di]
006368:  8B D8                        mov      bx, ax
00636A:  AD                           lodsw    ax, word ptr [si]
00636B:  0A C9                        or       cl, cl
00636D:  78 13                        js       0x6382
00636F:  F6 C5 01                     test     ch, 1
006372:  74 07                        je       0x637b
006374:  3B D0                        cmp      dx, ax
006376:  74 10                        je       0x6388
006378:  E9 B4 00                     jmp      0x642f
00637B:  3B D0                        cmp      dx, ax
00637D:  7F 09                        jg       0x6388
00637F:  E9 AD 00                     jmp      0x642f
006382:  3B C2                        cmp      ax, dx
006384:  0F 8D A7 00                  jge      0x642f
006388:  F6 C1 40                     test     cl, 0x40
00638B:  74 73                        je       0x6400
00638D:  B8 FF FF                     mov      ax, 0xffff
006390:  E8 00 FF                     call     0x6293
006393:  8A D5                        mov      dl, ch
006395:  80 E2 07                     and      dl, 7
006398:  75 42                        jne      0x63dc
00639A:  8B EE                        mov      bp, si
00639C:  32 D2                        xor      dl, dl
00639E:  AD                           lodsw    ax, word ptr [si]
00639F:  FE C2                        inc      dl
0063A1:  0B C0                        or       ax, ax
0063A3:  75 F9                        jne      0x639e
0063A5:  FE CA                        dec      dl
0063A7:  74 57                        je       0x6400
0063A9:  57                           push     di
0063AA:  65 8B 3E 46 67               mov      di, word ptr gs:[0x6746]
0063AF:  65 8B 1E 44 67               mov      bx, word ptr gs:[0x6744]
0063B4:  83 EB 02                     sub      bx, 2
0063B7:  83 E3 0F                     and      bx, 0xf
0063BA:  8B F5                        mov      si, bp
0063BC:  26 8B 01                     mov      ax, word ptr es:[bx + di]
0063BF:  3B 04                        cmp      ax, word ptr [si]
0063C1:  74 0C                        je       0x63cf
0063C3:  83 C6 02                     add      si, 2
0063C6:  F7 04 FF FF                  test     word ptr [si], 0xffff
0063CA:  75 F3                        jne      0x63bf
0063CC:  5F                           pop      di
0063CD:  EB 60                        jmp      0x642f
0063CF:  83 EB 02                     sub      bx, 2
0063D2:  83 E3 0F                     and      bx, 0xf
0063D5:  FE CA                        dec      dl
0063D7:  75 E1                        jne      0x63ba
0063D9:  5F                           pop      di
0063DA:  EB 24                        jmp      0x6400
0063DC:  AD                           lodsw    ax, word ptr [si]
0063DD:  0B C0                        or       ax, ax
0063DF:  74 4E                        je       0x642f
0063E1:  83 F8 FF                     cmp      ax, -1
0063E4:  74 49                        je       0x642f
0063E6:  65 8B 2E 46 67               mov      bp, word ptr gs:[0x6746]
0063EB:  B6 08                        mov      dh, 8
0063ED:  26 3B 46 00                  cmp      ax, word ptr es:[bp]
0063F1:  75 04                        jne      0x63f7
0063F3:  FE CA                        dec      dl
0063F5:  74 09                        je       0x6400
0063F7:  83 C5 02                     add      bp, 2
0063FA:  FE CE                        dec      dh
0063FC:  75 EF                        jne      0x63ed
0063FE:  EB DC                        jmp      0x63dc
006400:  F6 C1 20                     test     cl, 0x20
006403:  74 06                        je       0x640b
006405:  65 C6 06 B9 67 01            mov      byte ptr gs:[0x67b9], 1
00640B:  F6 C1 10                     test     cl, 0x10
00640E:  74 1C                        je       0x642c
006410:  B8 FF FF                     mov      ax, 0xffff
006413:  E8 7D FE                     call     0x6293
006416:  65 C6 06 B4 67 01            mov      byte ptr gs:[0x67b4], 1
00641C:  BD F8 67                     mov      bp, 0x67f8
00641F:  AD                           lodsw    ax, word ptr [si]
006420:  89 46 00                     mov      word ptr [bp], ax
006423:  0B C0                        or       ax, ax
006425:  74 05                        je       0x642c
006427:  83 C5 02                     add      bp, 2
00642A:  EB F3                        jmp      0x641f
00642C:  F9                           stc
00642D:  EB 01                        jmp      0x6430
00642F:  F8                           clc
006430:  5D                           pop      bp
006431:  5E                           pop      si
006432:  C3                           ret
