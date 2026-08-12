; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006863
; seg_off: 04da:14c3
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_shared_state_marker
; label_comment: Shared B1/B4/B5/B6/BE/BF/C0 handler: read a record word, resolve an immediate or C0/C2 record-backed RHS, then perform signed query comparisons or wrapping add/subtract/assignment
; incoming: vm_opcode_handlers:opcode_0xb1
; incoming: vm_opcode_handlers:opcode_0xb4
; incoming: vm_opcode_handlers:opcode_0xb5
; incoming: vm_opcode_handlers:opcode_0xb6
; incoming: vm_opcode_handlers:opcode_0xbe
; incoming: vm_opcode_handlers:opcode_0xbf
; incoming: vm_opcode_handlers:opcode_0xc0
; byte_count: 159
; boundary: cfg_blocks_26_terminals_10
; terminal: jmp 0x68db:6, jmp 0x68fd:2, jmp 0x6900:1, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: b2630958f693c2d37f25b2cdedb0420c7765dd7a03e9851497b34a371a272dec

006863:  57                           push     di
006864:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006869:  8B 1C                        mov      bx, word ptr [si]
00686B:  26 8B 09                     mov      cx, word ptr es:[bx + di]
00686E:  83 C6 02                     add      si, 2
006871:  AC                           lodsb    al, byte ptr [si]
006872:  8A E0                        mov      ah, al
006874:  AC                           lodsb    al, byte ptr [si]
006875:  8B 14                        mov      dx, word ptr [si]
006877:  3C C0                        cmp      al, 0xc0
006879:  74 04                        je       0x687f
00687B:  3C C2                        cmp      al, 0xc2
00687D:  75 07                        jne      0x6886
00687F:  57                           push     di
006880:  03 FA                        add      di, dx
006882:  26 8B 15                     mov      dx, word ptr es:[di]
006885:  5F                           pop      di
006886:  83 C6 02                     add      si, 2
006889:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
00688F:  74 53                        je       0x68e4
006891:  32 C0                        xor      al, al
006893:  80 FC F0                     cmp      ah, 0xf0
006896:  75 07                        jne      0x689f
006898:  3B CA                        cmp      cx, dx
00689A:  0F 95 C0                     setne    al
00689D:  EB 3C                        jmp      0x68db
00689F:  80 FC F3                     cmp      ah, 0xf3
0068A2:  75 07                        jne      0x68ab
0068A4:  3B CA                        cmp      cx, dx
0068A6:  0F 9E C0                     setle    al
0068A9:  EB 30                        jmp      0x68db
0068AB:  80 FC F4                     cmp      ah, 0xf4
0068AE:  75 07                        jne      0x68b7
0068B0:  3B CA                        cmp      cx, dx
0068B2:  0F 9D C0                     setge    al
0068B5:  EB 24                        jmp      0x68db
0068B7:  80 FC F1                     cmp      ah, 0xf1
0068BA:  75 07                        jne      0x68c3
0068BC:  3B CA                        cmp      cx, dx
0068BE:  0F 9C C0                     setl     al
0068C1:  EB 18                        jmp      0x68db
0068C3:  80 FC F2                     cmp      ah, 0xf2
0068C6:  75 07                        jne      0x68cf
0068C8:  3B CA                        cmp      cx, dx
0068CA:  0F 9F C0                     setg     al
0068CD:  EB 0C                        jmp      0x68db
0068CF:  80 FC F5                     cmp      ah, 0xf5
0068D2:  75 07                        jne      0x68db
0068D4:  3B CA                        cmp      cx, dx
0068D6:  0F 94 C0                     sete     al
0068D9:  EB 00                        jmp      0x68db
0068DB:  0A C0                        or       al, al
0068DD:  75 21                        jne      0x6900
0068DF:  E8 80 FB                     call     0x6462
0068E2:  EB 1C                        jmp      0x6900
0068E4:  80 FC F6                     cmp      ah, 0xf6
0068E7:  75 04                        jne      0x68ed
0068E9:  03 CA                        add      cx, dx
0068EB:  EB 10                        jmp      0x68fd
0068ED:  80 FC F7                     cmp      ah, 0xf7
0068F0:  75 04                        jne      0x68f6
0068F2:  2B CA                        sub      cx, dx
0068F4:  EB 07                        jmp      0x68fd
0068F6:  80 FC F5                     cmp      ah, 0xf5
0068F9:  75 02                        jne      0x68fd
0068FB:  8B CA                        mov      cx, dx
0068FD:  26 89 09                     mov      word ptr es:[bx + di], cx
006900:  5F                           pop      di
006901:  C3                           ret     
