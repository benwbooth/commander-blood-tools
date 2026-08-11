; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005afd
; seg_off: 04da:075d
; group: seg_04da
; provenance: recursive_graph
; label: vm_op_a3_collect
; label_comment: opcode 0xA3 handler: copy 0-term word list from COD into gs:0x67f8
; byte_count: 59
; boundary: cfg_blocks_8_terminals_2
; terminal: jmp 0x5b1a:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_005afd_vm_op_a3_collect.cpp
; routine_bytes_sha256: 56ddbb00ce9c9b47acf0470acd0fe6f485d40b0e419c4a173ef8ddaf4fb77570

005AFD:  50                           push     ax
005AFE:  06                           push     es
005AFF:  57                           push     di
005B00:  1E                           push     ds
005B01:  56                           push     si
005B02:  65 C5 36 20 67               lds      si, ptr gs:[0x6720]
005B07:  65 8B 36 72 67               mov      si, word ptr gs:[0x6772]
005B0C:  8A 04                        mov      al, byte ptr [si]
005B0E:  3C A3                        cmp      al, 0xa3
005B10:  75 20                        jne      0x5b32
005B12:  46                           inc      si
005B13:  8C E8                        mov      ax, gs
005B15:  8E C0                        mov      es, ax
005B17:  BF F8 67                     mov      di, 0x67f8
005B1A:  AD                           lodsw    ax, word ptr [si]
005B1B:  0B C0                        or       ax, ax
005B1D:  74 03                        je       0x5b22
005B1F:  AB                           stosw    word ptr es:[di], ax
005B20:  EB F8                        jmp      0x5b1a
005B22:  65 A1 70 67                  mov      ax, word ptr gs:[0x6770]
005B26:  0B C0                        or       ax, ax
005B28:  74 07                        je       0x5b31
005B2A:  AB                           stosw    word ptr es:[di], ax
005B2B:  33 C0                        xor      ax, ax
005B2D:  65 A3 70 67                  mov      word ptr gs:[0x6770], ax
005B31:  AB                           stosw    word ptr es:[di], ax
005B32:  5E                           pop      si
005B33:  1F                           pop      ds
005B34:  5F                           pop      di
005B35:  07                           pop      es
005B36:  58                           pop      ax
005B37:  C3                           ret     
