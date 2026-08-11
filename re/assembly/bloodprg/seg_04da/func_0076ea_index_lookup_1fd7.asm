; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0076ea
; seg_off: 04da:234a
; group: seg_04da
; provenance: static_dispatch_table_target
; label: index_lookup_1fd7
; label_comment: index-table lookup: di=0x1fd7; lodsb operand; cbw (signed index). Indexes a table at DS:0x1fd7 by the script operand
; incoming: byte_parser_dispatch_74e5:byte_0x0b
; byte_count: 106
; boundary: cfg_blocks_12_terminals_3
; terminal: jmp 0x76ff:1, jmp 0x7750:1, ret:1
; direct_callees: none
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_04da/func_0076ea_index_lookup_1fd7.cpp
; routine_bytes_sha256: ac97646e8463df2225f218348fcdd1694f485651b279fa2fc29b56d8efff43b7

0076EA:  1E                           push     ds
0076EB:  06                           push     es
0076EC:  57                           push     di
0076ED:  BF D7 1F                     mov      di, 0x1fd7
0076F0:  AC                           lodsb    al, byte ptr [si]
0076F1:  98                           cwde    
0076F2:  78 07                        js       0x76fb
0076F4:  48                           dec      ax
0076F5:  C1 E0 04                     shl      ax, 4
0076F8:  05 D7 0D                     add      ax, 0xdd7
0076FB:  AB                           stosw    word ptr es:[di], ax
0076FC:  BF 3A 21                     mov      di, 0x213a
0076FF:  AC                           lodsb    al, byte ptr [si]
007700:  0A C0                        or       al, al
007702:  78 07                        js       0x770b
007704:  3C 20                        cmp      al, 0x20
007706:  72 03                        jb       0x770b
007708:  AA                           stosb    byte ptr es:[di], al
007709:  EB F4                        jmp      0x76ff
00770B:  4E                           dec      si
00770C:  26 C6 05 00                  mov      byte ptr es:[di], 0
007710:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
007716:  75 38                        jne      0x7750
007718:  65 83 3E 58 0A FF            cmp      word ptr gs:[0xa58], -1
00771E:  74 13                        je       0x7733
007720:  56                           push     si
007721:  8C E8                        mov      ax, gs
007723:  8E D8                        mov      ds, ax
007725:  BE 37 21                     mov      si, 0x2137
007728:  9A 12 07 CE 01               lcall    0x1ce, 0x712
00772D:  66 33 C0                     xor      eax, eax
007730:  5E                           pop      si
007731:  EB 1D                        jmp      0x7750
007733:  65 83 3E 56 0A FF            cmp      word ptr gs:[0xa56], -1
007739:  74 15                        je       0x7750
00773B:  56                           push     si
00773C:  8C E8                        mov      ax, gs
00773E:  8E D8                        mov      ds, ax
007740:  BE 37 21                     mov      si, 0x2137
007743:  C4 3E 29 52                  les      di, ptr [0x5229]
007747:  9A 21 06 CE 01               lcall    0x1ce, 0x621
00774C:  66 33 C0                     xor      eax, eax
00774F:  5E                           pop      si
007750:  5F                           pop      di
007751:  07                           pop      es
007752:  1F                           pop      ds
007753:  C3                           ret     
