; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0067c8
; seg_off: 04da:1428
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a8_load_string
; label_comment: 0xA8 handler: copy NUL-terminated operand into buffer 0x2120 (bp), skip pad. THEN if buffer starts 'fin.' set gs:[0x67BD]=1 (fin/finale flag). THEN if !(gs:[0x67AA]&2) AND (gs:[0x24F3]&1 ship-active OR gs:[0x274F]&1): presentation request -> gs:[0x6788]=7 (active line), gs:[0x67AA]|=2, gs:[0x1FB2]=0, gs:[0x1FA3]=0xFFFF, gs:[0xB3B]=0. Port models only the string copy (VmEvent::LoadString); the fin-flag + presentation-request are engine/ship-presentation-flag coupled
; incoming: vm_opcode_handlers:opcode_0xa8
; byte_count: 104
; boundary: cfg_blocks_12_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 6f7818d61c8dcf7565c3ef19a51ddc3b38bb2331f58e7723ae69822a07aed6ea

0067C8:  BD 20 21                     mov      bp, 0x2120
0067CB:  AC                           lodsb    al, byte ptr [si]
0067CC:  88 46 00                     mov      byte ptr [bp], al
0067CF:  45                           inc      bp
0067D0:  0A C0                        or       al, al
0067D2:  75 F7                        jne      0x67cb
0067D4:  46                           inc      si
0067D5:  BD 20 21                     mov      bp, 0x2120
0067D8:  80 7E 00 66                  cmp      byte ptr [bp], 0x66
0067DC:  75 18                        jne      0x67f6
0067DE:  80 7E 01 69                  cmp      byte ptr [bp + 1], 0x69
0067E2:  75 12                        jne      0x67f6
0067E4:  80 7E 02 6E                  cmp      byte ptr [bp + 2], 0x6e
0067E8:  75 0C                        jne      0x67f6
0067EA:  80 7E 03 2E                  cmp      byte ptr [bp + 3], 0x2e
0067EE:  75 06                        jne      0x67f6
0067F0:  65 C6 06 BD 67 01            mov      byte ptr gs:[0x67bd], 1
0067F6:  65 F6 06 AA 67 02            test     byte ptr gs:[0x67aa], 2
0067FC:  75 31                        jne      0x682f
0067FE:  65 F7 06 F3 24 01 00         test     word ptr gs:[0x24f3], 1
006805:  75 08                        jne      0x680f
006807:  65 F6 06 4F 27 01            test     byte ptr gs:[0x274f], 1
00680D:  74 20                        je       0x682f
00680F:  65 C7 06 88 67 07 00         mov      word ptr gs:[0x6788], 7
006816:  65 80 0E AA 67 02            or       byte ptr gs:[0x67aa], 2
00681C:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
006822:  65 C7 06 A3 1F FF FF         mov      word ptr gs:[0x1fa3], 0xffff
006829:  65 C6 06 3B 0B 00            mov      byte ptr gs:[0xb3b], 0
00682F:  C3                           ret     
