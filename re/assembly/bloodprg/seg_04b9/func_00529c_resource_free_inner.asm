; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00529c
; seg_off: 04b9:010c
; group: seg_04b9
; provenance: recursive_graph
; label: resource_free_inner
; label_comment: SEG 0x4b9:0x10c: actual resource release (segment dealloc) called by resource_release once the loaded-flag check passes
; byte_count: 120
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x52dc:1, retf:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: d812c81a9fa76846cfd2bf73d0f72a16cfee1d0dd39b19427a3f26b902afcd00

00529C:  66 50                        push     eax
00529E:  53                           push     bx
00529F:  51                           push     cx
0052A0:  66 52                        push     edx
0052A2:  1E                           push     ds
0052A3:  56                           push     si
0052A4:  06                           push     es
0052A5:  57                           push     di
0052A6:  66 55                        push     ebp
0052A8:  8C E3                        mov      bx, fs
0052AA:  8E DB                        mov      ds, bx
0052AC:  8E C3                        mov      es, bx
0052AE:  8B D8                        mov      bx, ax
0052B0:  C1 E3 03                     shl      bx, 3
0052B3:  FF 37                        push     word ptr [bx]
0052B5:  83 67 02 FC                  and      word ptr [bx + 2], 0xfffc
0052B9:  66 8B 6F 04                  mov      ebp, dword ptr [bx + 4]
0052BD:  66 65 01 2E 46 0A            add      dword ptr gs:[0xa46], ebp
0052C3:  66 C1 ED 04                  shr      ebp, 4
0052C7:  65 29 2E 6A 0A               sub      word ptr gs:[0xa6a], bp
0052CC:  BF 00 08                     mov      di, 0x800
0052CF:  B9 00 01                     mov      cx, 0x100
0052D2:  F2 AF                        repne scasw ax, word ptr es:[di]
0052D4:  66 33 D2                     xor      edx, edx
0052D7:  8B F7                        mov      si, di
0052D9:  83 EF 02                     sub      di, 2
0052DC:  AD                           lodsw    ax, word ptr [si]
0052DD:  AB                           stosw    word ptr es:[di], ax
0052DE:  0B C0                        or       ax, ax
0052E0:  78 0D                        js       0x52ef
0052E2:  C1 E0 03                     shl      ax, 3
0052E5:  8B D8                        mov      bx, ax
0052E7:  29 2F                        sub      word ptr [bx], bp
0052E9:  66 03 57 04                  add      edx, dword ptr [bx + 4]
0052ED:  EB ED                        jmp      0x52dc
0052EF:  58                           pop      ax
0052F0:  66 0B D2                     or       edx, edx
0052F3:  74 12                        je       0x5307
0052F5:  8E C0                        mov      es, ax
0052F7:  33 FF                        xor      di, di
0052F9:  03 C5                        add      ax, bp
0052FB:  8E D8                        mov      ds, ax
0052FD:  33 F6                        xor      si, si
0052FF:  66 8B C2                     mov      eax, edx
005302:  9A 93 0B CE 01               lcall    0x1ce, 0xb93
005307:  66 5D                        pop      ebp
005309:  5F                           pop      di
00530A:  07                           pop      es
00530B:  5E                           pop      si
00530C:  1F                           pop      ds
00530D:  66 5A                        pop      edx
00530F:  59                           pop      cx
005310:  5B                           pop      bx
005311:  66 58                        pop      eax
005313:  CB                           retf    
