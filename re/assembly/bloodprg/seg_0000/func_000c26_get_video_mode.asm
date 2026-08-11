; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000c26
; seg_off: 0000:0626
; group: seg_0000
; provenance: recursive_graph
; label: get_video_mode
; label_comment: startup: int 10h ah=0x0f (get current video mode) -> gs:[0x5232]=al. Records the entry video mode for restore on exit
; byte_count: 154
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_0000/func_000c26_get_video_mode.cpp
; routine_bytes_sha256: 7f021e7db6e3efd3a5b2789397c1eb34d0d3c17681a33c612b40cf89585cefed

000C26:  50                           push     ax
000C27:  53                           push     bx
000C28:  51                           push     cx
000C29:  52                           push     dx
000C2A:  06                           push     es
000C2B:  57                           push     di
000C2C:  55                           push     bp
000C2D:  B4 0F                        mov      ah, 0xf
000C2F:  CD 10                        int      0x10
000C31:  65 A2 32 52                  mov      byte ptr gs:[0x5232], al
000C35:  33 C0                        xor      ax, ax
000C37:  B0 13                        mov      al, 0x13
000C39:  CD 10                        int      0x10
000C3B:  B8 30 11                     mov      ax, 0x1130
000C3E:  B7 03                        mov      bh, 3
000C40:  CD 10                        int      0x10
000C42:  65 89 2E 25 52               mov      word ptr gs:[0x5225], bp
000C47:  8C C0                        mov      ax, es
000C49:  65 A3 27 52                  mov      word ptr gs:[0x5227], ax
000C4D:  B8 40 00                     mov      ax, 0x40
000C50:  8E C0                        mov      es, ax
000C52:  26 A1 63 00                  mov      ax, word ptr es:[0x63]
000C56:  65 A3 9E 0A                  mov      word ptr gs:[0xa9e], ax
000C5A:  9A 16 00 99 02               lcall    0x299, 0x16
000C5F:  BA CE 03                     mov      dx, 0x3ce
000C62:  B0 05                        mov      al, 5
000C64:  EE                           out      dx, al
000C65:  42                           inc      dx
000C66:  EC                           in       al, dx
000C67:  24 EF                        and      al, 0xef
000C69:  EE                           out      dx, al
000C6A:  4A                           dec      dx
000C6B:  B0 06                        mov      al, 6
000C6D:  EE                           out      dx, al
000C6E:  42                           inc      dx
000C6F:  EC                           in       al, dx
000C70:  24 FD                        and      al, 0xfd
000C72:  EE                           out      dx, al
000C73:  BA C4 03                     mov      dx, 0x3c4
000C76:  B0 04                        mov      al, 4
000C78:  EE                           out      dx, al
000C79:  42                           inc      dx
000C7A:  EC                           in       al, dx
000C7B:  24 F7                        and      al, 0xf7
000C7D:  0C 04                        or       al, 4
000C7F:  EE                           out      dx, al
000C80:  65 8B 16 9E 0A               mov      dx, word ptr gs:[0xa9e]
000C85:  B0 14                        mov      al, 0x14
000C87:  EE                           out      dx, al
000C88:  42                           inc      dx
000C89:  EC                           in       al, dx
000C8A:  24 BF                        and      al, 0xbf
000C8C:  EE                           out      dx, al
000C8D:  4A                           dec      dx
000C8E:  B0 17                        mov      al, 0x17
000C90:  EE                           out      dx, al
000C91:  42                           inc      dx
000C92:  EC                           in       al, dx
000C93:  0C 40                        or       al, 0x40
000C95:  EE                           out      dx, al
000C96:  65 8B 16 9E 0A               mov      dx, word ptr gs:[0xa9e]
000C9B:  B0 11                        mov      al, 0x11
000C9D:  EE                           out      dx, al
000C9E:  42                           inc      dx
000C9F:  EC                           in       al, dx
000CA0:  0C 20                        or       al, 0x20
000CA2:  EE                           out      dx, al
000CA3:  BA C4 03                     mov      dx, 0x3c4
000CA6:  B8 02 0F                     mov      ax, 0xf02
000CA9:  EF                           out      dx, ax
000CAA:  B9 FF FF                     mov      cx, 0xffff
000CAD:  33 FF                        xor      di, di
000CAF:  B8 00 A0                     mov      ax, 0xa000
000CB2:  8E C0                        mov      es, ax
000CB4:  33 C0                        xor      ax, ax
000CB6:  F3 AA                        rep stosb byte ptr es:[di], al
000CB8:  5D                           pop      bp
000CB9:  5F                           pop      di
000CBA:  07                           pop      es
000CBB:  5A                           pop      dx
000CBC:  59                           pop      cx
000CBD:  5B                           pop      bx
000CBE:  58                           pop      ax
000CBF:  CB                           retf    
