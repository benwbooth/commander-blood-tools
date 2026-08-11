; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0056a6
; seg_off: 04da:0306
; group: seg_04da
; provenance: recursive_graph
; label: vm_script_block_scan
; label_comment: VM nested script-block scanner (2 calls): di=0x6eb0 (opcode handler table); lodsb opcode; scan to 0xff terminator - executes/skips a nested script block using the same dispatch as vm_exec_loop_dispatch. A block/subroutine executor
; byte_count: 88
; boundary: cfg_blocks_13_terminals_4
; terminal: jmp 0x56a9:2, jmp 0x56fd:1, ret:1
; direct_callees: 0x0062b6
; indirect_calls: 2
; routine_bytes_sha256: c4ff3c09a33b3bccb3f6a403f17144eddad34f62d8c07c42e1245e707474266c

0056A6:  BF B0 6E                     mov      di, 0x6eb0
0056A9:  AC                           lodsb    al, byte ptr [si]
0056AA:  3C FF                        cmp      al, 0xff
0056AC:  74 41                        je       0x56ef
0056AE:  8A D8                        mov      bl, al
0056B0:  80 EB A0                     sub      bl, 0xa0
0056B3:  78 3E                        js       0x56f3
0056B5:  80 FB 32                     cmp      bl, 0x32
0056B8:  7F 39                        jg       0x56f3
0056BA:  32 FF                        xor      bh, bh
0056BC:  03 DB                        add      bx, bx
0056BE:  65 C6 06 B4 67 00            mov      byte ptr gs:[0x67b4], 0
0056C4:  65 FF 11                     call     word ptr gs:[bx + di]
0056C7:  65 A0 B4 67                  mov      al, byte ptr gs:[0x67b4]
0056CB:  0A C0                        or       al, al
0056CD:  75 14                        jne      0x56e3
0056CF:  65 F6 06 AB 67 0F            test     byte ptr gs:[0x67ab], 0xf
0056D5:  74 D2                        je       0x56a9
0056D7:  E8 DC 0B                     call     0x62b6
0056DA:  65 FE 0E AB 67               dec      byte ptr gs:[0x67ab]
0056DF:  75 F6                        jne      0x56d7
0056E1:  EB C6                        jmp      0x56a9
0056E3:  FE C8                        dec      al
0056E5:  74 08                        je       0x56ef
0056E7:  65 C6 06 AB 67 00            mov      byte ptr gs:[0x67ab], 0
0056ED:  EB BA                        jmp      0x56a9
0056EF:  33 C0                        xor      ax, ax
0056F1:  EB 0A                        jmp      0x56fd
0056F3:  33 C0                        xor      ax, ax
0056F5:  9A 75 07 00 00               lcall    0, 0x775
0056FA:  B8 FF FF                     mov      ax, 0xffff
0056FD:  C3                           ret     
