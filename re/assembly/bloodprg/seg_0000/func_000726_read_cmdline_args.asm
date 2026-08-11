; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000726
; seg_off: 0000:0126
; group: seg_0000
; provenance: recursive_graph
; label: read_cmdline_args
; label_comment: startup: bp=0x23a (PSP/args area); cmp [bp],0; parse command-line arguments (the game needs 'AMR S162227 EMS WRIC:...' launch args, see memory). Reads the invocation config
; byte_count: 118
; boundary: cfg_blocks_13_terminals_5
; terminal: jmp 0x72d:1, jmp 0x785:1, jmp 0x797:2, ret:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_0000/func_000726_read_cmdline_args.cpp
; routine_bytes_sha256: 752826499995c9e2e5a43b58f7f47ef37bc9d52183464b45f197335382afa18f

000726:  55                           push     bp
000727:  51                           push     cx
000728:  57                           push     di
000729:  56                           push     si
00072A:  BD 3A 02                     mov      bp, 0x23a
00072D:  80 7E 00 00                  cmp      byte ptr [bp], 0
000731:  74 64                        je       0x797
000733:  B9 03 00                     mov      cx, 3
000736:  55                           push     bp
000737:  57                           push     di
000738:  8A 46 00                     mov      al, byte ptr [bp]
00073B:  26 3A 05                     cmp      al, byte ptr es:[di]
00073E:  75 14                        jne      0x754
000740:  45                           inc      bp
000741:  47                           inc      di
000742:  E2 F4                        loop     0x738
000744:  83 C4 04                     add      sp, 4
000747:  8A 46 00                     mov      al, byte ptr [bp]
00074A:  A8 01                        test     al, 1
00074C:  75 34                        jne      0x782
00074E:  A8 02                        test     al, 2
000750:  75 09                        jne      0x75b
000752:  EB 43                        jmp      0x797
000754:  5F                           pop      di
000755:  5D                           pop      bp
000756:  83 C5 05                     add      bp, 5
000759:  EB D2                        jmp      0x72d
00075B:  06                           push     es
00075C:  1E                           push     ds
00075D:  8B F7                        mov      si, di
00075F:  06                           push     es
000760:  1F                           pop      ds
000761:  32 DB                        xor      bl, bl
000763:  86 5C 03                     xchg     byte ptr [si + 3], bl
000766:  9A 32 03 CE 01               lcall    0x1ce, 0x332
00076B:  C1 E0 04                     shl      ax, 4
00076E:  80 EB 30                     sub      bl, 0x30
000771:  0A C3                        or       al, bl
000773:  65 A3 45 0C                  mov      word ptr gs:[0xc45], ax
000777:  8A 46 01                     mov      al, byte ptr [bp + 1]
00077A:  65 A2 3B 0C                  mov      byte ptr gs:[0xc3b], al
00077E:  1F                           pop      ds
00077F:  07                           pop      es
000780:  EB 15                        jmp      0x797
000782:  BD BA 01                     mov      bp, 0x1ba
000785:  26 8A 05                     mov      al, byte ptr es:[di]
000788:  0A C0                        or       al, al
00078A:  74 07                        je       0x793
00078C:  88 46 00                     mov      byte ptr [bp], al
00078F:  45                           inc      bp
000790:  47                           inc      di
000791:  EB F2                        jmp      0x785
000793:  C6 46 FF 00                  mov      byte ptr [bp - 1], 0
000797:  5E                           pop      si
000798:  5F                           pop      di
000799:  59                           pop      cx
00079A:  5D                           pop      bp
00079B:  C3                           ret     
