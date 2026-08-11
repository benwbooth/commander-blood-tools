; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006433
; seg_off: 05e3:0003
; group: seg_04da
; provenance: label_csv_target, manual_binary_boundary
; label: dic_word_lookup
; label_comment: dictionary lookup helper for A6 text assembly; compares a DIC word against directory names and returns the matched object offset in AX with CF as status
; byte_count: 47
; boundary: cfg_blocks_6_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 7a272aa1c6b9d71044b17fc182cce70eabef2dad9d0ae7ae6f028de29fad4dbc

006433:  06                           push     es
006434:  57                           push     di
006435:  1E                           push     ds
006436:  56                           push     si
006437:  65 C5 36 28 67               lds      si, ptr gs:[0x6728]
00643C:  65 C4 3E 2C 67               les      di, ptr gs:[0x672c]
006441:  03 F0                        add      si, ax
006443:  26 8B 45 12                  mov      ax, word ptr es:[di + 0x12]
006447:  83 F8 01                     cmp      ax, 1
00644A:  75 0C                        jne      0x6458
00644C:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
006451:  72 06                        jb       0x6459
006453:  83 C7 14                     add      di, 0x14
006456:  EB EB                        jmp      0x6443
006458:  F8                           clc
006459:  26 8B 45 10                  mov      ax, word ptr es:[di + 0x10]
00645D:  5E                           pop      si
00645E:  1F                           pop      ds
00645F:  5F                           pop      di
006460:  07                           pop      es
006461:  C3                           ret
