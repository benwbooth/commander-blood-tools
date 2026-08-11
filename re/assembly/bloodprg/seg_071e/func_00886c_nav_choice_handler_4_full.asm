; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00886c
; seg_off: 071e:108c
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_choice_handler_4_full
; label_comment: HANDLER 4 FULL DECODE (dispatch-table entry 4). Phases: [0x2565]&1 -> reset [0xADB], query-only layout prepass ([0x27E6] bracket around 0x8428); [0x2565]&2 -> interpolate 0x2AAB toward 0x253D via 0x8B:0xFAD, `jae` exits while incomplete, else clear the phase. Then the real 0x8428 call returns the selection; a NEGATIVE value exits. The selection dispatch is a CHAIN of `dec al / jns`, not a jump table: sel 0 -> [0x259B]=[0x259C]=1; sel 1 -> gated on [0xADE]&1, toggles the tablo2 voc via [0xBA3]/[0xBA0] and swaps the target-list pointer; sel 2 -> [0x2738]=1,[0x2736]=1 (left); sel 3 -> [0x2738]=1,[0x2737]=1 (right); sel 4 -> [0xB13]=2,[0xA3E]=0,[0xA40]=0. Tail always clears [0x2A19] and bit2 of [0x2793]. VERIFIED against ship3d.rs run_ship_3d_nav_choice_handler_4 -- exact match including the fall-through past selection 4 || MERGED 2026-07-25 (audit-fixes #130), also recorded as: navigation-choice handler 4 (dispatch-table entry 4) -- the largest: target-list toggle DS:0x2567/0x2569 between DS:0x2578 and DS:0x2581, the mu\\tablo2.voc sound gate, and the left/right/motion latches DS:0x2736/0x2737/0x2738. PORTED: ship3d.rs run_ship_3d_nav_choice_handler_4 || five-way handler: menu gate, tablo2 VOC toggle, left/right motion gates, sound gate
; incoming: nav_choice_subdispatch:choice_4
; byte_count: 247
; boundary: cfg_blocks_22_terminals_6
; terminal: jmp 0x8956:5, ret:1
; direct_callees: 0x008428
; indirect_calls: 3
; routine_bytes_sha256: 3314f6a97b79e0ccc745d5c06f0710051a6b79fb460dadc37fddcf50733f0e38

00886C:  06                           push     es
00886D:  8C E8                        mov      ax, gs
00886F:  8E C0                        mov      es, ax
008871:  BE 67 25                     mov      si, 0x2567
008874:  F6 06 65 25 01               test     byte ptr [0x2565], 1
008879:  74 21                        je       0x889c
00887B:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
008880:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
008885:  0E                           push     cs
008886:  E8 9F FB                     call     0x8428
008889:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
00888E:  FE 06 65 25                  inc      byte ptr [0x2565]
008892:  BE AB 2A                     mov      si, 0x2aab
008895:  BF CF 25                     mov      di, 0x25cf
008898:  66 A5                        movsd    dword ptr es:[di], dword ptr [si]
00889A:  66 A5                        movsd    dword ptr es:[di], dword ptr [si]
00889C:  F6 06 65 25 02               test     byte ptr [0x2565], 2
0088A1:  74 16                        je       0x88b9
0088A3:  56                           push     si
0088A4:  BE AB 2A                     mov      si, 0x2aab
0088A7:  BF 3D 25                     mov      di, 0x253d
0088AA:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
0088AF:  5E                           pop      si
0088B0:  0F 83 AD 00                  jae      0x8961
0088B4:  C6 06 65 25 00               mov      byte ptr [0x2565], 0
0088B9:  0E                           push     cs
0088BA:  E8 6B FB                     call     0x8428
0088BD:  0B C0                        or       ax, ax
0088BF:  0F 88 9E 00                  js       0x8961
0088C3:  FE C8                        dec      al
0088C5:  79 0D                        jns      0x88d4
0088C7:  C6 06 9B 25 01               mov      byte ptr [0x259b], 1
0088CC:  C6 06 9C 25 01               mov      byte ptr [0x259c], 1
0088D1:  E9 82 00                     jmp      0x8956
0088D4:  FE C8                        dec      al
0088D6:  79 4B                        jns      0x8923
0088D8:  F6 06 DE 0A 01               test     byte ptr [0xade], 1
0088DD:  74 77                        je       0x8956
0088DF:  F6 06 A3 0B 01               test     byte ptr [0xba3], 1
0088E4:  74 12                        je       0x88f8
0088E6:  C6 06 A0 0B 00               mov      byte ptr [0xba0], 0
0088EB:  C6 06 A3 0B 00               mov      byte ptr [0xba3], 0
0088F0:  B8 78 25                     mov      ax, 0x2578
0088F3:  A3 69 25                     mov      word ptr [0x2569], ax
0088F6:  EB 5E                        jmp      0x8956
0088F8:  C6 06 30 0D 00               mov      byte ptr [0xd30], 0
0088FD:  C6 06 A0 0B 00               mov      byte ptr [0xba0], 0
008902:  C6 06 A3 0B 01               mov      byte ptr [0xba3], 1
008907:  B8 81 25                     mov      ax, 0x2581
00890A:  A3 69 25                     mov      word ptr [0x2569], ax
00890D:  F6 06 DE 0A 01               test     byte ptr [0xade], 1
008912:  74 42                        je       0x8956
008914:  BE 3D 0D                     mov      si, 0xd3d
008917:  9A 07 06 1B 0B               lcall    0xb1b, 0x607
00891C:  9A 03 04 1B 0B               lcall    0xb1b, 0x403
008921:  EB 33                        jmp      0x8956
008923:  FE C8                        dec      al
008925:  79 0C                        jns      0x8933
008927:  C6 06 38 27 01               mov      byte ptr [0x2738], 1
00892C:  C6 06 36 27 01               mov      byte ptr [0x2736], 1
008931:  EB 23                        jmp      0x8956
008933:  FE C8                        dec      al
008935:  79 0C                        jns      0x8943
008937:  C6 06 38 27 01               mov      byte ptr [0x2738], 1
00893C:  C6 06 37 27 01               mov      byte ptr [0x2737], 1
008941:  EB 13                        jmp      0x8956
008943:  FE C8                        dec      al
008945:  79 0F                        jns      0x8956
008947:  C6 06 13 0B 02               mov      byte ptr [0xb13], 2
00894C:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
008951:  C6 06 40 0A 00               mov      byte ptr [0xa40], 0
008956:  C7 06 19 2A 00 00            mov      word ptr [0x2a19], 0
00895C:  80 26 93 27 FB               and      byte ptr [0x2793], 0xfb
008961:  07                           pop      es
008962:  C3                           ret     
