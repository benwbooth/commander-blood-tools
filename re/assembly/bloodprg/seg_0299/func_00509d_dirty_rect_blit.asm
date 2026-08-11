; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00509d
; seg_off: 0299:210d
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: dirty_rect_blit
; label_comment: SEG 0x299:0x210d: blit dirty rectangles from the LINEAR back-buffer (gs:0x5229) to the mode-X screen. Gated on dirty flag gs:[0x5231]&1. Per rect record es:[di]: +0x00=x(ax), +0x02=right/x2, +0x04=y(bx), +0x06=width(cx). Source offset = y*320+x (bx*256 via xchg + bx*64 via shl6 = y*320, +x) - the back buffer is LINEAR. Row-skip stride = 0x140-width. KEY: the game renders to a linear back-buffer then dirty-rect-blits to mode-X, so the engine's linear framebuffer matches the game's INTERNAL representation, not just address-equivalent
; incoming: call@0x00787f->0299:210d
; incoming: call@0x008ea0->0299:210d
; incoming: call@0x00b1d8->0299:210d
; byte_count: 231
; boundary: cfg_blocks_16_terminals_7
; terminal: jmp 0x50b4:1, jmp 0x5168:1, jmp 0x5176:4, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_00509d_dirty_rect_blit.cpp
; routine_bytes_sha256: 3b4e4e0ddaf931b6a3bdaac56a0d2dc34ab89aa36b42e0d61a4d151afcb97932

00509D:  06                           push     es
00509E:  1E                           push     ds
00509F:  56                           push     si
0050A0:  57                           push     di
0050A1:  50                           push     ax
0050A2:  53                           push     bx
0050A3:  51                           push     cx
0050A4:  52                           push     dx
0050A5:  65 F6 06 31 52 01            test     byte ptr gs:[0x5231], 1
0050AB:  0F 84 CC 00                  je       0x517b
0050AF:  65 C5 36 29 52               lds      si, ptr gs:[0x5229]
0050B4:  26 8B 05                     mov      ax, word ptr es:[di]
0050B7:  0B C0                        or       ax, ax
0050B9:  0F 88 BE 00                  js       0x517b
0050BD:  26 8B 5D 04                  mov      bx, word ptr es:[di + 4]
0050C1:  53                           push     bx
0050C2:  8B CB                        mov      cx, bx
0050C4:  86 DF                        xchg     bh, bl
0050C6:  C1 E1 06                     shl      cx, 6
0050C9:  03 D9                        add      bx, cx
0050CB:  03 D8                        add      bx, ax
0050CD:  8B F3                        mov      si, bx
0050CF:  26 8B 5D 02                  mov      bx, word ptr es:[di + 2]
0050D3:  2B D8                        sub      bx, ax
0050D5:  BA 40 01                     mov      dx, 0x140
0050D8:  2B D3                        sub      dx, bx
0050DA:  58                           pop      ax
0050DB:  26 8B 4D 06                  mov      cx, word ptr es:[di + 6]
0050DF:  2B C8                        sub      cx, ax
0050E1:  83 C7 08                     add      di, 8
0050E4:  06                           push     es
0050E5:  57                           push     di
0050E6:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
0050EB:  8B FE                        mov      di, si
0050ED:  8B C6                        mov      ax, si
0050EF:  8A E0                        mov      ah, al
0050F1:  8A C3                        mov      al, bl
0050F3:  25 03 03                     and      ax, 0x303
0050F6:  C1 EB 02                     shr      bx, 2
0050F9:  74 6D                        je       0x5168
0050FB:  2A C4                        sub      al, ah
0050FD:  80 DB 00                     sbb      bl, 0
005100:  75 06                        jne      0x5108
005102:  24 03                        and      al, 3
005104:  02 C4                        add      al, ah
005106:  EB 60                        jmp      0x5168
005108:  24 03                        and      al, 3
00510A:  74 32                        je       0x513e
00510C:  0A E4                        or       ah, ah
00510E:  74 19                        je       0x5129
005110:  8A F9                        mov      bh, cl
005112:  8A CC                        mov      cl, ah
005114:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
005116:  8A CB                        mov      cl, bl
005118:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00511B:  8A C8                        mov      cl, al
00511D:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00511F:  03 F2                        add      si, dx
005121:  03 FA                        add      di, dx
005123:  8A CF                        mov      cl, bh
005125:  E2 E9                        loop     0x5110
005127:  EB 4D                        jmp      0x5176
005129:  8A F9                        mov      bh, cl
00512B:  8A CB                        mov      cl, bl
00512D:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
005130:  8A C8                        mov      cl, al
005132:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
005134:  03 F2                        add      si, dx
005136:  03 FA                        add      di, dx
005138:  8A CF                        mov      cl, bh
00513A:  E2 ED                        loop     0x5129
00513C:  EB 38                        jmp      0x5176
00513E:  0A E4                        or       ah, ah
005140:  74 15                        je       0x5157
005142:  8A F9                        mov      bh, cl
005144:  8A CC                        mov      cl, ah
005146:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
005148:  8A CB                        mov      cl, bl
00514A:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00514D:  03 F2                        add      si, dx
00514F:  03 FA                        add      di, dx
005151:  8A CF                        mov      cl, bh
005153:  E2 ED                        loop     0x5142
005155:  EB 1F                        jmp      0x5176
005157:  8A F9                        mov      bh, cl
005159:  8A CB                        mov      cl, bl
00515B:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00515E:  03 F2                        add      si, dx
005160:  03 FA                        add      di, dx
005162:  8A CF                        mov      cl, bh
005164:  E2 F1                        loop     0x5157
005166:  EB 0E                        jmp      0x5176
005168:  8A F9                        mov      bh, cl
00516A:  8A C8                        mov      cl, al
00516C:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00516E:  03 F2                        add      si, dx
005170:  03 FA                        add      di, dx
005172:  8A CF                        mov      cl, bh
005174:  E2 F2                        loop     0x5168
005176:  5F                           pop      di
005177:  07                           pop      es
005178:  E9 39 FF                     jmp      0x50b4
00517B:  5A                           pop      dx
00517C:  59                           pop      cx
00517D:  5B                           pop      bx
00517E:  58                           pop      ax
00517F:  5F                           pop      di
005180:  5E                           pop      si
005181:  1F                           pop      ds
005182:  07                           pop      es
005183:  CB                           retf    
