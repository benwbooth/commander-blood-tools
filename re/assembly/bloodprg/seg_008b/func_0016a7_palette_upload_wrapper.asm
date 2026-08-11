; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0016a7
; seg_off: 008b:07f7
; group: seg_008b
; provenance: recursive_graph
; label: palette_upload_wrapper
; label_comment: palette upload: ds=es=gs; si=0x5b58 (game_palette_dac_buffer); lcall 0x299:0 -> vga_palette_write 0x2f90, which rep-outsb 768 bytes to the VGA DAC. Pushes the current game palette to hardware
; byte_count: 228
; boundary: cfg_blocks_17_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 6
; routine_bytes_sha256: 01da4bafc1ea757304d8e6d9f3498949e7a06568d2442a01d6411b83a94a374c

0016A7:  8C E8                        mov      ax, gs
0016A9:  8E D8                        mov      ds, ax
0016AB:  8E C0                        mov      es, ax
0016AD:  BE 58 5B                     mov      si, 0x5b58
0016B0:  9A 00 00 99 02               lcall    0x299, 0
0016B5:  33 C0                        xor      ax, ax
0016B7:  9A EB 0D 99 02               lcall    0x299, 0xdeb
0016BC:  BE 59 01                     mov      si, 0x159
0016BF:  B8 82 00                     mov      ax, 0x82
0016C2:  BB 60 00                     mov      bx, 0x60
0016C5:  B2 EF                        mov      dl, 0xef
0016C7:  B6 FF                        mov      dh, 0xff
0016C9:  9A D6 00 99 02               lcall    0x299, 0xd6
0016CE:  66 FF 36 19 52               push     dword ptr [0x5219]
0016D3:  66 A1 1D 52                  mov      eax, dword ptr [0x521d]
0016D7:  66 A3 19 52                  mov      dword ptr [0x5219], eax
0016DB:  66 33 C0                     xor      eax, eax
0016DE:  1E                           push     ds
0016DF:  C5 36 21 52                  lds      si, ptr [0x5221]
0016E3:  9A 3E 0F 99 02               lcall    0x299, 0xf3e
0016E8:  1F                           pop      ds
0016E9:  66 8F 06 19 52               pop      dword ptr [0x5219]
0016EE:  BA BA 01                     mov      dx, 0x1ba
0016F1:  B4 39                        mov      ah, 0x39
0016F3:  CD 21                        int      0x21
0016F5:  A0 BA 01                     mov      al, byte ptr [0x1ba]
0016F8:  2C 41                        sub      al, 0x41
0016FA:  A2 B8 01                     mov      byte ptr [0x1b8], al
0016FD:  B4 19                        mov      ah, 0x19
0016FF:  CD 21                        int      0x21
001701:  BF DA 01                     mov      di, 0x1da
001704:  A2 B9 01                     mov      byte ptr [0x1b9], al
001707:  04 41                        add      al, 0x41
001709:  AA                           stosb    byte ptr es:[di], al
00170A:  2C 40                        sub      al, 0x40
00170C:  8A D0                        mov      dl, al
00170E:  B0 3A                        mov      al, 0x3a
001710:  AA                           stosb    byte ptr es:[di], al
001711:  B0 5C                        mov      al, 0x5c
001713:  AA                           stosb    byte ptr es:[di], al
001714:  8B F7                        mov      si, di
001716:  B4 47                        mov      ah, 0x47
001718:  CD 21                        int      0x21
00171A:  BF FA 01                     mov      di, 0x1fa
00171D:  BE DA 01                     mov      si, 0x1da
001720:  AC                           lodsb    al, byte ptr [si]
001721:  AA                           stosb    byte ptr es:[di], al
001722:  0A C0                        or       al, al
001724:  75 FA                        jne      0x1720
001726:  4F                           dec      di
001727:  8B EF                        mov      bp, di
001729:  80 7E FF 5C                  cmp      byte ptr [bp - 1], 0x5c
00172D:  74 05                        je       0x1734
00172F:  C6 46 00 5C                  mov      byte ptr [bp], 0x5c
001733:  45                           inc      bp
001734:  BE BA 01                     mov      si, 0x1ba
001737:  BF 1A 02                     mov      di, 0x21a
00173A:  AC                           lodsb    al, byte ptr [si]
00173B:  AA                           stosb    byte ptr es:[di], al
00173C:  0A C0                        or       al, al
00173E:  75 FA                        jne      0x173a
001740:  4F                           dec      di
001741:  80 7D FF 5C                  cmp      byte ptr [di - 1], 0x5c
001745:  74 03                        je       0x174a
001747:  B0 5C                        mov      al, 0x5c
001749:  AA                           stosb    byte ptr es:[di], al
00174A:  BE 59 02                     mov      si, 0x259
00174D:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
001752:  B9 18 00                     mov      cx, 0x18
001755:  8B D6                        mov      dx, si
001757:  B4 4E                        mov      ah, 0x4e
001759:  CD 21                        int      0x21
00175B:  73 25                        jae      0x1782
00175D:  8B DE                        mov      bx, si
00175F:  8B CF                        mov      cx, di
001761:  8B FD                        mov      di, bp
001763:  AC                           lodsb    al, byte ptr [si]
001764:  AA                           stosb    byte ptr es:[di], al
001765:  0A C0                        or       al, al
001767:  75 FA                        jne      0x1763
001769:  8B F9                        mov      di, cx
00176B:  8B F3                        mov      si, bx
00176D:  AC                           lodsb    al, byte ptr [si]
00176E:  AA                           stosb    byte ptr es:[di], al
00176F:  0A C0                        or       al, al
001771:  75 FA                        jne      0x176d
001773:  BE FA 01                     mov      si, 0x1fa
001776:  BF 1A 02                     mov      di, 0x21a
001779:  9A 2F 05 CE 01               lcall    0x1ce, 0x52f
00177E:  8B F3                        mov      si, bx
001780:  8B F9                        mov      di, cx
001782:  83 C6 10                     add      si, 0x10
001785:  80 3C 00                     cmp      byte ptr [si], 0
001788:  75 C3                        jne      0x174d
00178A:  C3                           ret     
