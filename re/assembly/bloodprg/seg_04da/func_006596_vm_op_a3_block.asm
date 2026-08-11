; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006596
; seg_off: 04da:11f6
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a3_block
; label_comment: VM opcode 0xA3: conditional block - scans the script for the matching 0xA1 (POP) opcode, gated on gs:[0x67b2]&1 and comparing bytes to 0xa1. A structured block/if construct
; incoming: vm_opcode_handlers:opcode_0xa3
; byte_count: 69
; boundary: cfg_blocks_13_terminals_3
; terminal: jmp 0x65d9:2, ret:1
; direct_callees: 0x006293, 0x006462
; indirect_calls: 0
; routine_bytes_sha256: c575e6f7dd8a751259b763576fc96136bb0a3a23540c251e37ecb9db4cfeb000

006596:  57                           push     di
006597:  65 F6 06 B2 67 01            test     byte ptr gs:[0x67b2], 1
00659D:  75 35                        jne      0x65d4
00659F:  32 D2                        xor      dl, dl
0065A1:  8A 04                        mov      al, byte ptr [si]
0065A3:  3C A1                        cmp      al, 0xa1
0065A5:  75 03                        jne      0x65aa
0065A7:  46                           inc      si
0065A8:  FE C2                        inc      dl
0065AA:  BD 62 67                     mov      bp, 0x6762
0065AD:  AD                           lodsw    ax, word ptr [si]
0065AE:  65 F6 06 B1 67 02            test     byte ptr gs:[0x67b1], 2
0065B4:  74 03                        je       0x65b9
0065B6:  BD 64 67                     mov      bp, 0x6764
0065B9:  F7 46 00 FF FF               test     word ptr [bp], 0xffff
0065BE:  74 09                        je       0x65c9
0065C0:  3B 46 00                     cmp      ax, word ptr [bp]
0065C3:  74 09                        je       0x65ce
0065C5:  0A D2                        or       dl, dl
0065C7:  75 10                        jne      0x65d9
0065C9:  E8 96 FE                     call     0x6462
0065CC:  EB 0B                        jmp      0x65d9
0065CE:  0A D2                        or       dl, dl
0065D0:  75 F7                        jne      0x65c9
0065D2:  EB 05                        jmp      0x65d9
0065D4:  33 C0                        xor      ax, ax
0065D6:  E8 BA FC                     call     0x6293
0065D9:  5F                           pop      di
0065DA:  C3                           ret     
