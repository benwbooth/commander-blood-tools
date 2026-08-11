; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003ece
; seg_off: 0299:0f3e
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: blit_page_5219_gated
; label_comment: gated back-page blit: les di,gs:[0x5219]; bp=0x3e80; test byte gs:[0x252e],1. Copies into the back page only when the 0x252e enable bit is set
; incoming: call@0x0012b2->0299:0f3e
; incoming: call@0x0016e3->0299:0f3e
; incoming: call@0x001807->0299:0f3e
; incoming: call@0x001845->0299:0f3e
; incoming: call@0x001ef1->0299:0f3e
; incoming: call@0x001f69->0299:0f3e
; byte_count: 153
; boundary: cfg_blocks_5_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 3dea266474dc41ea3dd4f31b6d85b4a6196de6bb49c6989f64142e4bced98652

003ECE:  50                           push     ax
003ECF:  53                           push     bx
003ED0:  51                           push     cx
003ED1:  06                           push     es
003ED2:  57                           push     di
003ED3:  56                           push     si
003ED4:  55                           push     bp
003ED5:  1E                           push     ds
003ED6:  FC                           cld     
003ED7:  65 C4 3E 19 52               les      di, ptr gs:[0x5219]
003EDC:  BD 80 3E                     mov      bp, 0x3e80
003EDF:  65 F6 06 2E 25 01            test     byte ptr gs:[0x252e], 1
003EE5:  74 2B                        je       0x3f12
003EE7:  81 C7 F0 0A                  add      di, 0xaf0
003EEB:  81 C6 C0 2B                  add      si, 0x2bc0
003EEF:  BD A0 28                     mov      bp, 0x28a0
003EF2:  65 A1 27 25                  mov      ax, word ptr gs:[0x2527]
003EF6:  0B C0                        or       ax, ax
003EF8:  74 18                        je       0x3f12
003EFA:  8B D8                        mov      bx, ax
003EFC:  C1 E0 06                     shl      ax, 6
003EFF:  C1 E3 04                     shl      bx, 4
003F02:  03 C3                        add      ax, bx
003F04:  03 F8                        add      di, ax
003F06:  03 C0                        add      ax, ax
003F08:  2B E8                        sub      bp, ax
003F0A:  03 C0                        add      ax, ax
003F0C:  03 F0                        add      si, ax
003F0E:  0B ED                        or       bp, bp
003F10:  74 4C                        je       0x3f5e
003F12:  8B DF                        mov      bx, di
003F14:  56                           push     si
003F15:  BA C4 03                     mov      dx, 0x3c4
003F18:  B8 02 01                     mov      ax, 0x102
003F1B:  EF                           out      dx, ax
003F1C:  B8 03 00                     mov      ax, 3
003F1F:  8B CD                        mov      cx, bp
003F21:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003F22:  03 F0                        add      si, ax
003F24:  E2 FB                        loop     0x3f21
003F26:  8B FB                        mov      di, bx
003F28:  5E                           pop      si
003F29:  46                           inc      si
003F2A:  56                           push     si
003F2B:  B8 02 02                     mov      ax, 0x202
003F2E:  EF                           out      dx, ax
003F2F:  B8 03 00                     mov      ax, 3
003F32:  8B CD                        mov      cx, bp
003F34:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003F35:  03 F0                        add      si, ax
003F37:  E2 FB                        loop     0x3f34
003F39:  8B FB                        mov      di, bx
003F3B:  5E                           pop      si
003F3C:  46                           inc      si
003F3D:  56                           push     si
003F3E:  B8 02 04                     mov      ax, 0x402
003F41:  EF                           out      dx, ax
003F42:  B8 03 00                     mov      ax, 3
003F45:  8B CD                        mov      cx, bp
003F47:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003F48:  03 F0                        add      si, ax
003F4A:  E2 FB                        loop     0x3f47
003F4C:  8B FB                        mov      di, bx
003F4E:  5E                           pop      si
003F4F:  46                           inc      si
003F50:  B8 02 08                     mov      ax, 0x802
003F53:  EF                           out      dx, ax
003F54:  B8 03 00                     mov      ax, 3
003F57:  8B CD                        mov      cx, bp
003F59:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003F5A:  03 F0                        add      si, ax
003F5C:  E2 FB                        loop     0x3f59
003F5E:  1F                           pop      ds
003F5F:  5D                           pop      bp
003F60:  5E                           pop      si
003F61:  5F                           pop      di
003F62:  07                           pop      es
003F63:  59                           pop      cx
003F64:  5B                           pop      bx
003F65:  58                           pop      ax
003F66:  CB                           retf    
