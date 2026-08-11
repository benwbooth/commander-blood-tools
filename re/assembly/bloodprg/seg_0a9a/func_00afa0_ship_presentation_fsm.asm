; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00afa0
; seg_off: 0a9a:0000
; group: seg_0a9a
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: ship_presentation_fsm
; label_comment: ship/navigation presentation FSM; dispatches ship HUD, procedural 3D, and dialogue update branches
; incoming: call@0x00127b->0a9a:0000
; byte_count: 217
; boundary: cfg_blocks_23_terminals_6
; terminal: jmp 0xb076:5, retf:1
; direct_callees: 0x00b079, 0x00b34e, 0x00b6dd, 0x00b75c
; indirect_calls: 4
; routine_bytes_sha256: 097da2c66843f677d4d07cd154d36336f0a000e7fac1d2d575eb53e9c7bcfa34

00AFA0:  1E                           push     ds
00AFA1:  56                           push     si
00AFA2:  A1 F3 24                     mov      ax, word ptr [0x24f3]
00AFA5:  A8 01                        test     al, 1
00AFA7:  0F 84 CB 00                  je       0xb076
00AFAB:  C7 06 49 52 01 00            mov      word ptr [0x5249], 1
00AFB1:  A9 1E 00                     test     ax, 0x1e
00AFB4:  75 34                        jne      0xafea
00AFB6:  B8 04 00                     mov      ax, 4
00AFB9:  9A 41 12 99 02               lcall    0x299, 0x1241
00AFBE:  B8 1F 00                     mov      ax, 0x1f
00AFC1:  9A 41 12 99 02               lcall    0x299, 0x1241
00AFC6:  C7 06 93 27 00 00            mov      word ptr [0x2793], 0
00AFCC:  80 0E F3 24 02               or       byte ptr [0x24f3], 2
00AFD1:  C7 06 F5 24 04 00            mov      word ptr [0x24f5], 4
00AFD7:  C6 06 2D 25 00               mov      byte ptr [0x252d], 0
00AFDC:  C7 06 27 25 00 00            mov      word ptr [0x2527], 0
00AFE2:  C6 06 2F 25 00               mov      byte ptr [0x252f], 0
00AFE7:  E9 8C 00                     jmp      0xb076
00AFEA:  E8 6F 07                     call     0xb75c
00AFED:  0E                           push     cs
00AFEE:  E8 EC 06                     call     0xb6dd
00AFF1:  9A 00 00 71 09               lcall    0x971, 0
00AFF6:  A8 02                        test     al, 2
00AFF8:  74 30                        je       0xb02a
00AFFA:  F6 06 34 25 01               test     byte ptr [0x2534], 1
00AFFF:  75 1E                        jne      0xb01f
00B001:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
00B006:  75 6E                        jne      0xb076
00B008:  A1 F5 24                     mov      ax, word ptr [0x24f5]
00B00B:  0B C0                        or       ax, ax
00B00D:  74 10                        je       0xb01f
00B00F:  A3 88 67                     mov      word ptr [0x6788], ax
00B012:  40                           inc      ax
00B013:  83 F8 06                     cmp      ax, 6
00B016:  75 02                        jne      0xb01a
00B018:  33 C0                        xor      ax, ax
00B01A:  A3 F5 24                     mov      word ptr [0x24f5], ax
00B01D:  EB 57                        jmp      0xb076
00B01F:  C6 06 34 25 00               mov      byte ptr [0x2534], 0
00B024:  C7 06 F3 24 05 00            mov      word ptr [0x24f3], 5
00B02A:  A8 04                        test     al, 4
00B02C:  74 13                        je       0xb041
00B02E:  F6 06 35 25 01               test     byte ptr [0x2535], 1
00B033:  74 07                        je       0xb03c
00B035:  83 3E 4F 52 64               cmp      word ptr [0x524f], 0x64
00B03A:  75 3A                        jne      0xb076
00B03C:  E8 3A 00                     call     0xb079
00B03F:  EB 35                        jmp      0xb076
00B041:  A8 08                        test     al, 8
00B043:  74 2A                        je       0xb06f
00B045:  F6 06 D8 27 01               test     byte ptr [0x27d8], 1
00B04A:  74 0F                        je       0xb05b
00B04C:  C7 06 F3 24 11 00            mov      word ptr [0x24f3], 0x11
00B052:  33 C0                        xor      ax, ax
00B054:  9A EB 0D 99 02               lcall    0x299, 0xdeb
00B059:  EB 1B                        jmp      0xb076
00B05B:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
00B060:  75 14                        jne      0xb076
00B062:  C7 06 88 67 03 00            mov      word ptr [0x6788], 3
00B068:  C6 06 D8 27 00               mov      byte ptr [0x27d8], 0
00B06D:  EB 07                        jmp      0xb076
00B06F:  A8 10                        test     al, 0x10
00B071:  74 03                        je       0xb076
00B073:  E8 D8 02                     call     0xb34e
00B076:  5E                           pop      si
00B077:  1F                           pop      ds
00B078:  CB                           retf    
