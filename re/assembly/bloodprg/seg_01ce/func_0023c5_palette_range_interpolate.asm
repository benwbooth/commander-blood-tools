; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0023c5
; seg_off: 01ce:00e5
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: palette_range_interpolate
; label_comment: Interpolates inclusive RGB palette entries BX..DX from DS:SI toward caller ES:DI by signed percent AL, writing the live GS:0x5251 palette
; incoming: call@0x001fb0->01ce:00e5
; byte_count: 104
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: cc5aa75531534b4ad46005afe9348397b3c8b309bbf9d2e5885007eb9c24c130

0023C5:  0F A0                        push     fs
0023C7:  55                           push     bp
0023C8:  51                           push     cx
0023C9:  56                           push     si
0023CA:  06                           push     es
0023CB:  57                           push     di
0023CC:  8B EF                        mov      bp, di
0023CE:  06                           push     es
0023CF:  0F A1                        pop      fs
0023D1:  BF 51 52                     mov      di, 0x5251
0023D4:  0F A8                        push     gs
0023D6:  07                           pop      es
0023D7:  2B D3                        sub      dx, bx
0023D9:  8B CB                        mov      cx, bx
0023DB:  E3 0C                        jcxz     0x23e9
0023DD:  50                           push     ax
0023DE:  B0 03                        mov      al, 3
0023E0:  F6 E1                        mul      cl
0023E2:  03 F8                        add      di, ax
0023E4:  03 F0                        add      si, ax
0023E6:  03 E8                        add      bp, ax
0023E8:  58                           pop      ax
0023E9:  8B CA                        mov      cx, dx
0023EB:  41                           inc      cx
0023EC:  8B D8                        mov      bx, ax
0023EE:  B7 64                        mov      bh, 0x64
0023F0:  AC                           lodsb    al, byte ptr [si]
0023F1:  64 8A 56 00                  mov      dl, byte ptr fs:[bp]
0023F5:  45                           inc      bp
0023F6:  2A C2                        sub      al, dl
0023F8:  F6 EB                        imul     bl
0023FA:  F6 FF                        idiv     bh
0023FC:  02 D0                        add      dl, al
0023FE:  8A C2                        mov      al, dl
002400:  AA                           stosb    byte ptr es:[di], al
002401:  AC                           lodsb    al, byte ptr [si]
002402:  64 8A 56 00                  mov      dl, byte ptr fs:[bp]
002406:  45                           inc      bp
002407:  2A C2                        sub      al, dl
002409:  F6 EB                        imul     bl
00240B:  F6 FF                        idiv     bh
00240D:  02 D0                        add      dl, al
00240F:  8A C2                        mov      al, dl
002411:  AA                           stosb    byte ptr es:[di], al
002412:  AC                           lodsb    al, byte ptr [si]
002413:  64 8A 56 00                  mov      dl, byte ptr fs:[bp]
002417:  45                           inc      bp
002418:  2A C2                        sub      al, dl
00241A:  F6 EB                        imul     bl
00241C:  F6 FF                        idiv     bh
00241E:  02 D0                        add      dl, al
002420:  8A C2                        mov      al, dl
002422:  AA                           stosb    byte ptr es:[di], al
002423:  E2 CB                        loop     0x23f0
002425:  5F                           pop      di
002426:  07                           pop      es
002427:  5E                           pop      si
002428:  59                           pop      cx
002429:  5D                           pop      bp
00242A:  0F A1                        pop      fs
00242C:  CB                           retf    
