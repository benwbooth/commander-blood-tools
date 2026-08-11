; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007ce8
; seg_off: 071e:0508
; group: seg_071e
; provenance: recursive_graph
; label: list_walk_f18
; label_comment: list walk: ds=ax; si=[0xf18]; loop lodsw until 0. Walks a null-word-terminated list pointed to by [0xf18]
; byte_count: 147
; boundary: cfg_blocks_11_terminals_2
; terminal: jmp 0x7d0c:1, ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 92dc92ecf871a930484a022ef1e87302750ec7bd825a8a071e7f8a127ef44ebd

007CE8:  50                           push     ax
007CE9:  53                           push     bx
007CEA:  52                           push     dx
007CEB:  51                           push     cx
007CEC:  1E                           push     ds
007CED:  56                           push     si
007CEE:  57                           push     di
007CEF:  55                           push     bp
007CF0:  8C E8                        mov      ax, gs
007CF2:  8E D8                        mov      ds, ax
007CF4:  8B 36 18 0F                  mov      si, word ptr [0xf18]
007CF8:  AD                           lodsw    ax, word ptr [si]
007CF9:  0B C0                        or       ax, ax
007CFB:  78 75                        js       0x7d72
007CFD:  3B 06 1C 13                  cmp      ax, word ptr [0x131c]
007D01:  7F 6F                        jg       0x7d72
007D03:  33 C9                        xor      cx, cx
007D05:  8B D9                        mov      bx, cx
007D07:  8B FE                        mov      di, si
007D09:  BD F2 0A                     mov      bp, 0xaf2
007D0C:  AC                           lodsb    al, byte ptr [si]
007D0D:  43                           inc      bx
007D0E:  0A C0                        or       al, al
007D10:  74 20                        je       0x7d32
007D12:  3C 20                        cmp      al, 0x20
007D14:  75 F6                        jne      0x7d0c
007D16:  80 FB 1C                     cmp      bl, 0x1c
007D19:  7C F1                        jl       0x7d0c
007D1B:  89 5E 00                     mov      word ptr [bp], bx
007D1E:  41                           inc      cx
007D1F:  C1 E3 02                     shl      bx, 2
007D22:  81 EB A0 00                  sub      bx, 0xa0
007D26:  F7 DB                        neg      bx
007D28:  89 5E 02                     mov      word ptr [bp + 2], bx
007D2B:  83 C5 04                     add      bp, 4
007D2E:  33 DB                        xor      bx, bx
007D30:  EB DA                        jmp      0x7d0c
007D32:  87 F7                        xchg     di, si
007D34:  41                           inc      cx
007D35:  4B                           dec      bx
007D36:  89 5E 00                     mov      word ptr [bp], bx
007D39:  C1 E3 02                     shl      bx, 2
007D3C:  81 EB A0 00                  sub      bx, 0xa0
007D40:  F7 DB                        neg      bx
007D42:  89 5E 02                     mov      word ptr [bp + 2], bx
007D45:  BD F2 0A                     mov      bp, 0xaf2
007D48:  BB 6E 00                     mov      bx, 0x6e
007D4B:  B2 EF                        mov      dl, 0xef
007D4D:  8B 46 02                     mov      ax, word ptr [bp + 2]
007D50:  8A 76 00                     mov      dh, byte ptr [bp]
007D53:  9A D6 00 99 02               lcall    0x299, 0xd6
007D58:  83 C3 08                     add      bx, 8
007D5B:  83 C5 04                     add      bp, 4
007D5E:  E2 ED                        loop     0x7d4d
007D60:  8B F7                        mov      si, di
007D62:  8B 04                        mov      ax, word ptr [si]
007D64:  0B C0                        or       ax, ax
007D66:  78 0A                        js       0x7d72
007D68:  3B 06 1C 13                  cmp      ax, word ptr [0x131c]
007D6C:  7F 04                        jg       0x7d72
007D6E:  89 36 18 0F                  mov      word ptr [0xf18], si
007D72:  5D                           pop      bp
007D73:  5F                           pop      di
007D74:  5E                           pop      si
007D75:  1F                           pop      ds
007D76:  59                           pop      cx
007D77:  5A                           pop      dx
007D78:  5B                           pop      bx
007D79:  58                           pop      ax
007D7A:  C3                           ret     
