; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007d7b
; seg_off: 071e:059b
; group: seg_071e
; provenance: recursive_graph
; label: nav_actor_slot_update_loop
; label_comment: walks six 0x18-byte navigation actor/object slots and dispatches via cs:0x06d4
; byte_count: 161
; boundary: cfg_blocks_12_terminals_2
; terminal: jmp 0x7e04:1, ret:1
; direct_callees: 0x008269
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_071e/func_007d7b_nav_actor_slot_update_loop.cpp
; routine_bytes_sha256: 10e3dc30894e83264c391791e90722465d66152d6f51dea830c0acb9132f37ec

007D7B:  50                           push     ax
007D7C:  53                           push     bx
007D7D:  51                           push     cx
007D7E:  52                           push     dx
007D7F:  55                           push     bp
007D80:  56                           push     si
007D81:  06                           push     es
007D82:  57                           push     di
007D83:  A0 AC 67                     mov      al, byte ptr [0x67ac]
007D86:  0A 06 B2 1F                  or       al, byte ptr [0x1fb2]
007D8A:  0A 06 65 25                  or       al, byte ptr [0x2565]
007D8E:  0A 06 36 27                  or       al, byte ptr [0x2736]
007D92:  0A 06 37 27                  or       al, byte ptr [0x2737]
007D96:  0B 06 19 2A                  or       ax, word ptr [0x2a19]
007D9A:  0A 06 E7 27                  or       al, byte ptr [0x27e7]
007D9E:  0A 06 DA 27                  or       al, byte ptr [0x27da]
007DA2:  0A 06 13 0B                  or       al, byte ptr [0xb13]
007DA6:  75 6B                        jne      0x7e13
007DA8:  B9 06 00                     mov      cx, 6
007DAB:  BD 1B 2A                     mov      bp, 0x2a1b
007DAE:  8B 46 00                     mov      ax, word ptr [bp]
007DB1:  A8 01                        test     al, 1
007DB3:  74 4F                        je       0x7e04
007DB5:  A8 04                        test     al, 4
007DB7:  74 0A                        je       0x7dc3
007DB9:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
007DBE:  C6 06 40 0A 00               mov      byte ptr [0xa40], 0
007DC3:  8B F5                        mov      si, bp
007DC5:  83 C6 0C                     add      si, 0xc
007DC8:  E8 9E 04                     call     0x8269
007DCB:  F6 46 00 08                  test     byte ptr [bp], 8
007DCF:  74 17                        je       0x7de8
007DD1:  A1 95 27                     mov      ax, word ptr [0x2795]
007DD4:  03 C0                        add      ax, ax
007DD6:  3B 46 0A                     cmp      ax, word ptr [bp + 0xa]
007DD9:  74 0D                        je       0x7de8
007DDB:  8B 46 0A                     mov      ax, word ptr [bp + 0xa]
007DDE:  A3 9B 27                     mov      word ptr [0x279b], ax
007DE1:  83 0E 93 27 08               or       word ptr [0x2793], 8
007DE6:  EB 1C                        jmp      0x7e04
007DE8:  F6 46 00 02                  test     byte ptr [bp], 2
007DEC:  74 16                        je       0x7e04
007DEE:  A1 95 27                     mov      ax, word ptr [0x2795]
007DF1:  03 C0                        add      ax, ax
007DF3:  3B 46 0A                     cmp      ax, word ptr [bp + 0xa]
007DF6:  74 0C                        je       0x7e04
007DF8:  C6 46 00 01                  mov      byte ptr [bp], 1
007DFC:  B8 04 00                     mov      ax, 4
007DFF:  9A 41 12 99 02               lcall    0x299, 0x1241
007E04:  8B D9                        mov      bx, cx
007E06:  4B                           dec      bx
007E07:  03 DB                        add      bx, bx
007E09:  2E FF 97 D4 06               call     word ptr cs:[bx + 0x6d4]
007E0E:  83 C5 18                     add      bp, 0x18
007E11:  E2 9B                        loop     0x7dae
007E13:  5F                           pop      di
007E14:  07                           pop      es
007E15:  5E                           pop      si
007E16:  5D                           pop      bp
007E17:  5A                           pop      dx
007E18:  59                           pop      cx
007E19:  5B                           pop      bx
007E1A:  58                           pop      ax
007E1B:  C3                           ret     
