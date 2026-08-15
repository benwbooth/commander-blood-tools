; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000813
; seg_off: 0000:0213
; group: seg_0000
; provenance: installed_interrupt_vector, manual_binary_boundary
; label: bloodprg_timer_isr
; label_comment: INT 08h handler installed by 0x00079C. Services game countdowns from the 200 Hz PIT, chains the saved BIOS timer handler every eleventh interrupt, and acknowledges the PIC on intervening interrupts.
; incoming: int21_setvect_08@0x0007b9->0000:0213
; byte_count: 296
; boundary: cfg_blocks_36_terminals_3
; terminal: iret:1, jmp 0x905:1, ljmp gs:[0xb1d]:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: c1470a6e3c0b6d0e7d672d516891ff8fcb10c8679293bc5d03064dac23f9205a

000813:  50                           push     ax
000814:  65 F6 06 21 0B 01            test     byte ptr gs:[0xb21], 1
00081A:  0F 84 11 01                  je       0x92f
00081E:  56                           push     si
00081F:  53                           push     bx
000820:  65 F6 06 DF 0A 01            test     byte ptr gs:[0xadf], 1
000826:  0F 85 F6 00                  jne      0x920
00082A:  65 C6 06 23 0B 00            mov      byte ptr gs:[0xb23], 0
000830:  BE 29 0B                     mov      si, 0xb29
000833:  65 8B 44 04                  mov      ax, word ptr gs:[si + 4]
000837:  0B C0                        or       ax, ax
000839:  74 05                        je       0x840
00083B:  48                           dec      ax
00083C:  65 89 44 04                  mov      word ptr gs:[si + 4], ax
000840:  65 8B 04                     mov      ax, word ptr gs:[si]
000843:  40                           inc      ax
000844:  65 89 04                     mov      word ptr gs:[si], ax
000847:  65 83 54 02 00               adc      word ptr gs:[si + 2], 0
00084C:  D1 E8                        shr      ax, 1
00084E:  0F 82 CE 00                  jb       0x920
000852:  83 C6 06                     add      si, 6
000855:  65 8B 1C                     mov      bx, word ptr gs:[si]
000858:  0B DB                        or       bx, bx
00085A:  74 04                        je       0x860
00085C:  4B                           dec      bx
00085D:  65 89 1C                     mov      word ptr gs:[si], bx
000860:  D1 E8                        shr      ax, 1
000862:  0F 82 BA 00                  jb       0x920
000866:  83 C6 02                     add      si, 2
000869:  65 8B 1C                     mov      bx, word ptr gs:[si]
00086C:  0B DB                        or       bx, bx
00086E:  74 04                        je       0x874
000870:  4B                           dec      bx
000871:  65 89 1C                     mov      word ptr gs:[si], bx
000874:  D1 E8                        shr      ax, 1
000876:  0F 82 A6 00                  jb       0x920
00087A:  83 C6 02                     add      si, 2
00087D:  65 8B 1C                     mov      bx, word ptr gs:[si]
000880:  0B DB                        or       bx, bx
000882:  74 04                        je       0x888
000884:  4B                           dec      bx
000885:  65 89 1C                     mov      word ptr gs:[si], bx
000888:  65 FF 0E 27 0B               dec      word ptr gs:[0xb27]
00088D:  75 3F                        jne      0x8ce
00088F:  65 FF 06 3B 0B               inc      word ptr gs:[0xb3b]
000894:  65 83 3E 5A 67 00            cmp      word ptr gs:[0x675a], 0
00089A:  75 1E                        jne      0x8ba
00089C:  56                           push     si
00089D:  1E                           push     ds
00089E:  50                           push     ax
00089F:  51                           push     cx
0008A0:  8C E8                        mov      ax, gs
0008A2:  8E D8                        mov      ds, ax
0008A4:  BE DE 6A                     mov      si, 0x6ade
0008A7:  B9 1E 00                     mov      cx, 0x1e
0008AA:  AD                           lodsw    ax, word ptr [si]
0008AB:  0B C0                        or       ax, ax
0008AD:  74 05                        je       0x8b4
0008AF:  78 03                        js       0x8b4
0008B1:  FF 4C FE                     dec      word ptr [si - 2]
0008B4:  E2 F4                        loop     0x8aa
0008B6:  59                           pop      cx
0008B7:  58                           pop      ax
0008B8:  1F                           pop      ds
0008B9:  5E                           pop      si
0008BA:  65 C7 06 27 0B 19 00         mov      word ptr gs:[0xb27], 0x19
0008C1:  65 8B 5C 06                  mov      bx, word ptr gs:[si + 6]
0008C5:  0B DB                        or       bx, bx
0008C7:  74 05                        je       0x8ce
0008C9:  4B                           dec      bx
0008CA:  65 89 5C 06                  mov      word ptr gs:[si + 6], bx
0008CE:  D1 E8                        shr      ax, 1
0008D0:  72 4E                        jb       0x920
0008D2:  65 FE 06 3F 0B               inc      byte ptr gs:[0xb3f]
0008D7:  83 C6 02                     add      si, 2
0008DA:  65 8B 1C                     mov      bx, word ptr gs:[si]
0008DD:  0B DB                        or       bx, bx
0008DF:  74 04                        je       0x8e5
0008E1:  4B                           dec      bx
0008E2:  65 89 1C                     mov      word ptr gs:[si], bx
0008E5:  D1 E8                        shr      ax, 1
0008E7:  72 37                        jb       0x920
0008E9:  65 8A 26 19 0B               mov      ah, byte ptr gs:[0xb19]
0008EE:  F6 C4 01                     test     ah, 1
0008F1:  74 19                        je       0x90c
0008F3:  E4 61                        in       al, 0x61
0008F5:  F6 C4 02                     test     ah, 2
0008F8:  75 07                        jne      0x901
0008FA:  0C 03                        or       al, 3
0008FC:  80 CC 02                     or       ah, 2
0008FF:  EB 04                        jmp      0x905
000901:  24 FC                        and      al, 0xfc
000903:  32 E4                        xor      ah, ah
000905:  E6 61                        out      0x61, al
000907:  65 88 26 19 0B               mov      byte ptr gs:[0xb19], ah
00090C:  65 C6 06 23 0B 01            mov      byte ptr gs:[0xb23], 1
000912:  83 C6 02                     add      si, 2
000915:  65 8B 1C                     mov      bx, word ptr gs:[si]
000918:  0B DB                        or       bx, bx
00091A:  74 04                        je       0x920
00091C:  4B                           dec      bx
00091D:  65 89 1C                     mov      word ptr gs:[si], bx
000920:  5B                           pop      bx
000921:  5E                           pop      si
000922:  65 FE 0E 22 0B               dec      byte ptr gs:[0xb22]
000927:  75 0C                        jne      0x935
000929:  65 C6 06 22 0B 0B            mov      byte ptr gs:[0xb22], 0xb
00092F:  58                           pop      ax
000930:  65 FF 2E 1D 0B               ljmp     gs:[0xb1d]
000935:  B0 20                        mov      al, 0x20
000937:  E6 20                        out      0x20, al
000939:  58                           pop      ax
00093A:  CF                           iret
