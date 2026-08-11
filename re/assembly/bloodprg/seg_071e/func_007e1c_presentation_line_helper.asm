; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007e1c
; seg_off: 0781:000c
; group: seg_071e
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: presentation_line_helper
; label_comment: presentation/dialogue helper gated by [0x2793]&8 and record flag bit 2; advances or completes a presentation line record and returns status in CF
; incoming: call@0x007ee9->0x007e1c
; incoming: call@0x007f6c->0x007e1c
; incoming: call@0x007fd4->0x007e1c
; incoming: call@0x008053->0x007e1c
; incoming: call@0x0080cc->0x007e1c
; incoming: call@0x008153->0x007e1c
; incoming: call@0x0081c0->0x007e1c
; incoming: call@0x00822d->0x007e1c
; byte_count: 152
; boundary: cfg_blocks_13_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 2
; routine_bytes_sha256: 73adf983beab60796f0f8075ee37a5e5d0a7ecc96c48979cbca01418d70bce6a

007E1C:  50                           push     ax
007E1D:  53                           push     bx
007E1E:  51                           push     cx
007E1F:  52                           push     dx
007E20:  56                           push     si
007E21:  F6 06 93 27 08               test     byte ptr [0x2793], 8
007E26:  0F 85 84 00                  jne      0x7eae
007E2A:  F6 46 00 04                  test     byte ptr [bp], 4
007E2E:  75 3C                        jne      0x7e6c
007E30:  80 0E 93 27 04               or       byte ptr [0x2793], 4
007E35:  8B 46 02                     mov      ax, word ptr [bp + 2]
007E38:  C4 3E 80 0A                  les      di, ptr [0xa80]
007E3C:  1E                           push     ds
007E3D:  8C E3                        mov      bx, fs
007E3F:  8E DB                        mov      ds, bx
007E41:  BE 04 0C                     mov      si, 0xc04
007E44:  C1 E0 04                     shl      ax, 4
007E47:  03 F0                        add      si, ax
007E49:  9A DB 07 CE 01               lcall    0x1ce, 0x7db
007E4E:  1F                           pop      ds
007E4F:  26 8B 45 02                  mov      ax, word ptr es:[di + 2]
007E53:  89 46 06                     mov      word ptr [bp + 6], ax
007E56:  48                           dec      ax
007E57:  F6 06 E4 27 01               test     byte ptr [0x27e4], 1
007E5C:  75 07                        jne      0x7e65
007E5E:  33 C0                        xor      ax, ax
007E60:  C6 06 E4 27 00               mov      byte ptr [0x27e4], 0
007E65:  89 46 08                     mov      word ptr [bp + 8], ax
007E68:  80 4E 00 04                  or       byte ptr [bp], 4
007E6C:  55                           push     bp
007E6D:  C4 3E 80 0A                  les      di, ptr [0xa80]
007E71:  B8 04 00                     mov      ax, 4
007E74:  8B 5E 14                     mov      bx, word ptr [bp + 0x14]
007E77:  8B 4E 16                     mov      cx, word ptr [bp + 0x16]
007E7A:  8B 6E 08                     mov      bp, word ptr [bp + 8]
007E7D:  9A BE 11 99 02               lcall    0x299, 0x11be
007E82:  5D                           pop      bp
007E83:  F6 06 E4 27 01               test     byte ptr [0x27e4], 1
007E88:  74 0A                        je       0x7e94
007E8A:  8B 46 08                     mov      ax, word ptr [bp + 8]
007E8D:  0B C0                        or       ax, ax
007E8F:  74 12                        je       0x7ea3
007E91:  48                           dec      ax
007E92:  EB 09                        jmp      0x7e9d
007E94:  8B 46 08                     mov      ax, word ptr [bp + 8]
007E97:  3B 46 06                     cmp      ax, word ptr [bp + 6]
007E9A:  74 07                        je       0x7ea3
007E9C:  40                           inc      ax
007E9D:  89 46 08                     mov      word ptr [bp + 8], ax
007EA0:  F8                           clc
007EA1:  EB 0B                        jmp      0x7eae
007EA3:  C6 06 E4 27 00               mov      byte ptr [0x27e4], 0
007EA8:  80 26 93 27 FB               and      byte ptr [0x2793], 0xfb
007EAD:  F9                           stc
007EAE:  5E                           pop      si
007EAF:  5A                           pop      dx
007EB0:  59                           pop      cx
007EB1:  5B                           pop      bx
007EB2:  58                           pop      ax
007EB3:  C3                           ret
