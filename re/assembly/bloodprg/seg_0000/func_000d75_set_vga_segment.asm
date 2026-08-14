; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000d75
; seg_off: 0000:0775
; group: seg_0000
; provenance: relocation_proven_far_transfer_target
; label: error_overlay_draw
; label_comment: centered French coding/file/allocation error overlay. Temporarily sets the GS:0x5221 display pointer segment to A000, renders one to three 4x5 text rows, includes caller DS:DX for file errors, and formats signed FS handle/GS free-byte values for allocation errors. Direct vectors: re/tools/oracle_vectors/func_0d75_natural.json
; incoming: call@0x005275->0000:0775
; incoming: call@0x005696->0000:0775
; incoming: call@0x0056f5->0000:0775
; byte_count: 237
; boundary: cfg_blocks_7_terminals_3
; terminal: jmp 0xe53:2, retf:1
; direct_callees: 0x000e62
; indirect_calls: 14
; routine_bytes_sha256: a03c615631c6c929440cfafa741891a1b0bdb4dc0b4b8a1c6b15607ec3f9e3a4

000D75:  66 50                        push     eax
000D77:  53                           push     bx
000D78:  51                           push     cx
000D79:  52                           push     dx
000D7A:  06                           push     es
000D7B:  57                           push     di
000D7C:  1E                           push     ds
000D7D:  56                           push     si
000D7E:  65 FF 36 23 52               push     word ptr gs:[0x5223]
000D83:  65 C7 06 23 52 00 A0         mov      word ptr gs:[0x5223], 0xa000
000D8A:  0B C0                        or       ax, ax
000D8C:  75 22                        jne      0xdb0
000D8E:  8C E8                        mov      ax, gs
000D90:  8E C0                        mov      es, ax
000D92:  8E D8                        mov      ds, ax
000D94:  BE 2E 00                     mov      si, 0x2e
000D97:  8B FE                        mov      di, si
000D99:  9A 85 03 CE 01               lcall    0x1ce, 0x385
000D9E:  BB 01 00                     mov      bx, 1
000DA1:  0E                           push     cs
000DA2:  E8 BD 00                     call     0xe62
000DA5:  BA 0F 00                     mov      dx, 0xf
000DA8:  9A 5A 07 99 02               lcall    0x299, 0x75a
000DAD:  E9 A3 00                     jmp      0xe53
000DB0:  83 F8 01                     cmp      ax, 1
000DB3:  75 2D                        jne      0xde2
000DB5:  1E                           push     ds
000DB6:  52                           push     dx
000DB7:  8C E8                        mov      ax, gs
000DB9:  8E C0                        mov      es, ax
000DBB:  8E D8                        mov      ds, ax
000DBD:  BE 41 00                     mov      si, 0x41
000DC0:  8B FE                        mov      di, si
000DC2:  9A 85 03 CE 01               lcall    0x1ce, 0x385
000DC7:  BB 02 00                     mov      bx, 2
000DCA:  0E                           push     cs
000DCB:  E8 94 00                     call     0xe62
000DCE:  BA 0F 00                     mov      dx, 0xf
000DD1:  9A 5A 07 99 02               lcall    0x299, 0x75a
000DD6:  83 C3 06                     add      bx, 6
000DD9:  5E                           pop      si
000DDA:  1F                           pop      ds
000DDB:  9A 5A 07 99 02               lcall    0x299, 0x75a
000DE0:  EB 71                        jmp      0xe53
000DE2:  83 F8 02                     cmp      ax, 2
000DE5:  75 6C                        jne      0xe53
000DE7:  8C E8                        mov      ax, gs
000DE9:  8E C0                        mov      es, ax
000DEB:  8E D8                        mov      ds, ax
000DED:  BE 55 00                     mov      si, 0x55
000DF0:  8B FE                        mov      di, si
000DF2:  9A 85 03 CE 01               lcall    0x1ce, 0x385
000DF7:  BB 03 00                     mov      bx, 3
000DFA:  0E                           push     cs
000DFB:  E8 64 00                     call     0xe62
000DFE:  BA 0F 00                     mov      dx, 0xf
000E01:  9A 5A 07 99 02               lcall    0x299, 0x75a
000E06:  83 C3 06                     add      bx, 6
000E09:  BE 73 00                     mov      si, 0x73
000E0C:  9A 5A 07 99 02               lcall    0x299, 0x75a
000E11:  8B C8                        mov      cx, ax
000E13:  8B FE                        mov      di, si
000E15:  9A 85 03 CE 01               lcall    0x1ce, 0x385
000E1A:  C1 E0 02                     shl      ax, 2
000E1D:  03 C1                        add      ax, cx
000E1F:  50                           push     ax
000E20:  BF F2 0A                     mov      di, 0xaf2
000E23:  64 A1 00 0C                  mov      ax, word ptr fs:[0xc00]
000E27:  9A D2 01 CE 01               lcall    0x1ce, 0x1d2
000E2C:  58                           pop      ax
000E2D:  8B F7                        mov      si, di
000E2F:  9A 5A 07 99 02               lcall    0x299, 0x75a
000E34:  83 C3 06                     add      bx, 6
000E37:  BE 7D 00                     mov      si, 0x7d
000E3A:  91                           xchg     cx, ax
000E3B:  9A 5A 07 99 02               lcall    0x299, 0x75a
000E40:  66 65 A1 46 0A               mov      eax, dword ptr gs:[0xa46]
000E45:  9A 0B 02 CE 01               lcall    0x1ce, 0x20b
000E4A:  8B F7                        mov      si, di
000E4C:  8B C1                        mov      ax, cx
000E4E:  9A 5A 07 99 02               lcall    0x299, 0x75a
000E53:  65 8F 06 23 52               pop      word ptr gs:[0x5223]
000E58:  5E                           pop      si
000E59:  1F                           pop      ds
000E5A:  5F                           pop      di
000E5B:  07                           pop      es
000E5C:  5A                           pop      dx
000E5D:  59                           pop      cx
000E5E:  5B                           pop      bx
000E5F:  66 58                        pop      eax
000E61:  CB                           retf    
