; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0026cf
; seg_off: 01ce:03ef
; group: seg_01ce
; provenance: recursive_graph
; label: resource_archive_match
; label_comment: stage the embedded index through small EMS, small XMS, or its DOS cache; mask the mutable DS:SI name, scan packed 25-byte records, and publish/seek a matching payload
; byte_count: 244
; boundary: cfg_blocks_17_terminals_4
; terminal: jmp 0x275c:2, jmp 0x2775:1, ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: e85483839d72827a794ff588012ff18a7a86800448d894873f7cfefcca60875b

0026CF:  1E                           push     ds
0026D0:  51                           push     cx
0026D1:  52                           push     dx
0026D2:  65 83 3E 86 0A 00            cmp      word ptr gs:[0xa86], 0
0026D8:  0F 84 E3 00                  je       0x27bf
0026DC:  1E                           push     ds
0026DD:  56                           push     si
0026DE:  65 83 3E 64 0A FF            cmp      word ptr gs:[0xa64], -1
0026E4:  74 20                        je       0x2706
0026E6:  33 C0                        xor      ax, ax
0026E8:  8B D8                        mov      bx, ax
0026EA:  65 8B 16 64 0A               mov      dx, word ptr gs:[0xa64]
0026EF:  B9 04 00                     mov      cx, 4
0026F2:  B4 44                        mov      ah, 0x44
0026F4:  CD 67                        int      0x67
0026F6:  FE C3                        inc      bl
0026F8:  FE C0                        inc      al
0026FA:  E2 F6                        loop     0x26f2
0026FC:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
002701:  BE 02 00                     mov      si, 2
002704:  EB 56                        jmp      0x275c
002706:  65 C5 16 BC 0A               lds      dx, ptr gs:[0xabc]
00270B:  81 C2 00 7D                  add      dx, 0x7d00
00270F:  65 83 3E 62 0A FF            cmp      word ptr gs:[0xa62], -1
002715:  74 36                        je       0x274d
002717:  BF 6C 0A                     mov      di, 0xa6c
00271A:  8B F7                        mov      si, di
00271C:  66 B8 00 7D 00 00            mov      eax, 0x7d00
002722:  66 AB                        stosd    dword ptr es:[di], eax
002724:  65 A1 62 0A                  mov      ax, word ptr gs:[0xa62]
002728:  AB                           stosw    word ptr es:[di], ax
002729:  66 33 C0                     xor      eax, eax
00272C:  66 AB                        stosd    dword ptr es:[di], eax
00272E:  33 C0                        xor      ax, ax
002730:  AB                           stosw    word ptr es:[di], ax
002731:  8B C2                        mov      ax, dx
002733:  AB                           stosw    word ptr es:[di], ax
002734:  8C D8                        mov      ax, ds
002736:  AB                           stosw    word ptr es:[di], ax
002737:  8C E8                        mov      ax, gs
002739:  8E D8                        mov      ds, ax
00273B:  B8 00 0B                     mov      ax, 0xb00
00273E:  FF 1E 4A 0A                  lcall    [0xa4a]
002742:  65 C5 36 BC 0A               lds      si, ptr gs:[0xabc]
002747:  81 C6 02 7D                  add      si, 0x7d02
00274B:  EB 0F                        jmp      0x275c
00274D:  65 8B 1E 88 0A               mov      bx, word ptr gs:[0xa88]
002752:  B9 FF FF                     mov      cx, 0xffff
002755:  B4 3F                        mov      ah, 0x3f
002757:  CD 21                        int      0x21
002759:  BE 02 7D                     mov      si, 0x7d02
00275C:  5F                           pop      di
00275D:  07                           pop      es
00275E:  8B D6                        mov      dx, si
002760:  8B DF                        mov      bx, di
002762:  26 8A 05                     mov      al, byte ptr es:[di]
002765:  3C 61                        cmp      al, 0x61
002767:  72 05                        jb       0x276e
002769:  24 DF                        and      al, 0xdf
00276B:  26 88 05                     mov      byte ptr es:[di], al
00276E:  47                           inc      di
00276F:  0A C0                        or       al, al
002771:  75 EF                        jne      0x2762
002773:  33 C0                        xor      ax, ax
002775:  8B FB                        mov      di, bx
002777:  38 24                        cmp      byte ptr [si], ah
002779:  74 44                        je       0x27bf
00277B:  8A 04                        mov      al, byte ptr [si]
00277D:  26 0A 05                     or       al, byte ptr es:[di]
002780:  74 0A                        je       0x278c
002782:  A6                           cmpsb    byte ptr [si], byte ptr es:[di]
002783:  74 F6                        je       0x277b
002785:  83 C2 19                     add      dx, 0x19
002788:  8B F2                        mov      si, dx
00278A:  EB E9                        jmp      0x2775
00278C:  8B F2                        mov      si, dx
00278E:  65 C6 06 E2 0A 01            mov      byte ptr gs:[0xae2], 1
002794:  65 8B 1E 86 0A               mov      bx, word ptr gs:[0xa86]
002799:  66 8B 44 10                  mov      eax, dword ptr [si + 0x10]
00279D:  66 65 A3 8E 0A               mov      dword ptr gs:[0xa8e], eax
0027A2:  66 65 A3 92 0A               mov      dword ptr gs:[0xa92], eax
0027A7:  8B 54 14                     mov      dx, word ptr [si + 0x14]
0027AA:  8B 4C 16                     mov      cx, word ptr [si + 0x16]
0027AD:  65 89 16 8A 0A               mov      word ptr gs:[0xa8a], dx
0027B2:  65 89 0E 8C 0A               mov      word ptr gs:[0xa8c], cx
0027B7:  B8 00 42                     mov      ax, 0x4200
0027BA:  CD 21                        int      0x21
0027BC:  66 33 C0                     xor      eax, eax
0027BF:  5A                           pop      dx
0027C0:  59                           pop      cx
0027C1:  1F                           pop      ds
0027C2:  C3                           ret     
