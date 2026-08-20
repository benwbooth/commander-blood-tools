; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00218d
; seg_off: 008b:12dd
; group: seg_008b
; provenance: input_action_handler_table_index_1
; label: input_action_move_next
; label_comment: Moves the VM-directory selection downward while respecting the 20-byte entry terminator and 15-row viewport, or moves through editable save slots and copies the selected name.
; byte_count: 116
; boundary: cfg_blocks_13_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 33054a8ce9a76051c99544a3028d3aefb393f5848753f12d8b69e7c453548548

00218D:  1E                           push     ds
00218E:  56                           push     si
00218F:  51                           push     cx
002190:  F6 06 A6 67 03               test     byte ptr [0x67a6], 3
002195:  74 42                        je       0x21d9
002197:  8C E8                        mov      ax, gs
002199:  8E D8                        mov      ds, ax
00219B:  A1 9E 67                     mov      ax, word ptr [0x679e]
00219E:  83 F8 FF                     cmp      ax, -1
0021A1:  75 5A                        jne      0x21fd
0021A3:  A1 A2 67                     mov      ax, word ptr [0x67a2]
0021A6:  40                           inc      ax
0021A7:  8B D8                        mov      bx, ax
0021A9:  BA 14 00                     mov      dx, 0x14
0021AC:  F6 E2                        mul      dl
0021AE:  BE 80 6F                     mov      si, 0x6f80
0021B1:  F6 06 A6 67 01               test     byte ptr [0x67a6], 1
0021B6:  74 04                        je       0x21bc
0021B8:  C5 36 2C 67                  lds      si, dword ptr [0x672c]
0021BC:  03 F0                        add      si, ax
0021BE:  8B 4C 12                     mov      cx, word ptr [si + 0x12]
0021C1:  E3 3A                        jcxz     0x21fd
0021C3:  65 89 1E A2 67               mov      word ptr gs:[0x67a2], bx
0021C8:  65 2B 1E A0 67               sub      bx, word ptr gs:[0x67a0]
0021CD:  83 FB 0F                     cmp      bx, 0xf
0021D0:  7C 2B                        jl       0x21fd
0021D2:  65 FF 06 A0 67               inc      word ptr gs:[0x67a0]
0021D7:  EB 24                        jmp      0x21fd
0021D9:  F6 06 36 27 01               test     byte ptr [0x2736], 1
0021DE:  74 1D                        je       0x21fd
0021E0:  83 3E 32 27 08               cmp      word ptr [0x2732], 8
0021E5:  74 16                        je       0x21fd
0021E7:  FF 06 32 27                  inc      word ptr [0x2732]
0021EB:  83 06 34 27 20               add      word ptr [0x2734], 0x20
0021F0:  8B 36 34 27                  mov      si, word ptr [0x2734]
0021F4:  B9 04 00                     mov      cx, 4
0021F7:  BF 3B 27                     mov      di, 0x273b
0021FA:  F3 66 A5                     rep movsd
0021FD:  59                           pop      cx
0021FE:  5E                           pop      si
0021FF:  1F                           pop      ds
002200:  C3                           ret
