; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003066
; seg_off: 0299:00d6
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: blit_dst_row_addr_5221
; label_comment: blit dest-row address: les di,gs:[0x5221] (display page); bx<<6 + <<4 = y*80 row stride. Computes the destination scanline address (sibling of 0x3033)
; incoming: call@0x0016c9->0299:00d6
; incoming: call@0x007d53->0299:00d6
; byte_count: 103
; boundary: cfg_blocks_6_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 1a6318b0c4bc03d39b77a5255c764e5f4f3c47d04b302bb3f833a0a091bdbaa9

003066:  50                           push     ax
003067:  53                           push     bx
003068:  51                           push     cx
003069:  52                           push     dx
00306A:  06                           push     es
00306B:  57                           push     di
00306C:  1E                           push     ds
00306D:  0F A0                        push     fs
00306F:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
003074:  8B CB                        mov      cx, bx
003076:  86 E9                        xchg     cl, ch
003078:  C1 E3 06                     shl      bx, 6
00307B:  03 CB                        add      cx, bx
00307D:  03 C1                        add      ax, cx
00307F:  03 F8                        add      di, ax
003081:  8B DE                        mov      bx, si
003083:  8C D8                        mov      ax, ds
003085:  8E E0                        mov      fs, ax
003087:  64 8A 07                     mov      al, byte ptr fs:[bx]
00308A:  0A C0                        or       al, al
00308C:  74 33                        je       0x30c1
00308E:  57                           push     di
00308F:  65 C5 36 25 52               lds      si, ptr gs:[0x5225]
003094:  32 E4                        xor      ah, ah
003096:  C1 E0 03                     shl      ax, 3
003099:  03 F0                        add      si, ax
00309B:  B9 08 00                     mov      cx, 8
00309E:  51                           push     cx
00309F:  8A 24                        mov      ah, byte ptr [si]
0030A1:  B9 08 00                     mov      cx, 8
0030A4:  8A C2                        mov      al, dl
0030A6:  D0 E4                        shl      ah, 1
0030A8:  73 03                        jae      0x30ad
0030AA:  26 88 05                     mov      byte ptr es:[di], al
0030AD:  47                           inc      di
0030AE:  E2 F4                        loop     0x30a4
0030B0:  59                           pop      cx
0030B1:  46                           inc      si
0030B2:  81 C7 38 01                  add      di, 0x138
0030B6:  E2 E6                        loop     0x309e
0030B8:  43                           inc      bx
0030B9:  5F                           pop      di
0030BA:  83 C7 08                     add      di, 8
0030BD:  FE CE                        dec      dh
0030BF:  75 C6                        jne      0x3087
0030C1:  8B F3                        mov      si, bx
0030C3:  0F A1                        pop      fs
0030C5:  1F                           pop      ds
0030C6:  5F                           pop      di
0030C7:  07                           pop      es
0030C8:  5A                           pop      dx
0030C9:  59                           pop      cx
0030CA:  5B                           pop      bx
0030CB:  58                           pop      ax
0030CC:  CB                           retf    
