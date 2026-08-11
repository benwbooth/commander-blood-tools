; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000cef
; seg_off: 0000:06ef
; group: seg_0000
; provenance: recursive_graph
; label: mouse_reset_hide
; label_comment: mouse init: int 33h ax=0 (reset driver), int 33h ax=2 (hide cursor). Initializes the mouse driver with cursor hidden (the game draws its own cursor)
; byte_count: 31
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 93fcb283730c84a7e6d09b0f04862d9ca68850294f97a8ff107f627150a4999b

000CEF:  50                           push     ax
000CF0:  53                           push     bx
000CF1:  51                           push     cx
000CF2:  52                           push     dx
000CF3:  06                           push     es
000CF4:  33 C0                        xor      ax, ax
000CF6:  CD 33                        int      0x33
000CF8:  B8 02 00                     mov      ax, 2
000CFB:  CD 33                        int      0x33
000CFD:  B9 0C 00                     mov      cx, 0xc
000D00:  BA 0C 00                     mov      dx, 0xc
000D03:  B8 0F 00                     mov      ax, 0xf
000D06:  CD 33                        int      0x33
000D08:  07                           pop      es
000D09:  5A                           pop      dx
000D0A:  59                           pop      cx
000D0B:  5B                           pop      bx
000D0C:  58                           pop      ax
000D0D:  CB                           retf    
