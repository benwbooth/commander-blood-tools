; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001f78
; seg_off: 008b:10c8
; group: seg_008b
; provenance: recursive_graph
; label: transition_progress_524
; label_comment: transition/fade progress counter: ax=[0x524f]; cmp 0x64 (100 = complete); else add [0x524d] (step). A 0..100 progress counter driving a fade/scroll/transition
; byte_count: 68
; boundary: cfg_blocks_5_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: fcf9037274a0cac487a1ce94f3feed6c8a699d01aaaec22163ddf555d8e6368c

001F78:  50                           push     ax
001F79:  57                           push     di
001F7A:  56                           push     si
001F7B:  53                           push     bx
001F7C:  52                           push     dx
001F7D:  06                           push     es
001F7E:  A1 4F 52                     mov      ax, word ptr [0x524f]
001F81:  83 F8 64                     cmp      ax, 0x64
001F84:  74 2F                        je       0x1fb5
001F86:  03 06 4D 52                  add      ax, word ptr [0x524d]
001F8A:  83 F8 64                     cmp      ax, 0x64
001F8D:  7E 03                        jle      0x1f92
001F8F:  B8 64 00                     mov      ax, 0x64
001F92:  C6 06 55 5B 01               mov      byte ptr [0x5b55], 1
001F97:  A3 4F 52                     mov      word ptr [0x524f], ax
001F9A:  8C EB                        mov      bx, gs
001F9C:  8E C3                        mov      es, bx
001F9E:  BF 51 55                     mov      di, 0x5551
001FA1:  BE 51 58                     mov      si, 0x5851
001FA4:  33 DB                        xor      bx, bx
001FA6:  8B D3                        mov      dx, bx
001FA8:  8A 1E 51 5B                  mov      bl, byte ptr [0x5b51]
001FAC:  8A 16 52 5B                  mov      dl, byte ptr [0x5b52]
001FB0:  9A E5 00 CE 01               lcall    0x1ce, 0xe5
001FB5:  07                           pop      es
001FB6:  5A                           pop      dx
001FB7:  5B                           pop      bx
001FB8:  5E                           pop      si
001FB9:  5F                           pop      di
001FBA:  58                           pop      ax
001FBB:  CB                           retf    
