; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00224d
; seg_off: 008b:139d
; group: seg_008b
; provenance: input_action_handler_table_index_7
; label: input_action_cancel
; label_comment: Escape always leaves pause. When presentation, dialogue, ship, and line gates allow cancellation it rewinds the resource source, resets the queue, clears the first 384 palette bytes, and marks the palette dirty; otherwise it latches Escape for the active UI.
; byte_count: 101
; boundary: cfg_blocks_10_terminals_1
; terminal: ret:1
; direct_callees: 0x0022d0, 0x00a757
; indirect_calls: 0
; routine_bytes_sha256: 4978dfa9eddbcaaca95aab48168cb64904776bf96260fde5bffd2f65bf00d75e

00224D:  66 50                        push     eax
00224F:  51                           push     cx
002250:  C6 06 DF 0A 00               mov      byte ptr [0xadf], 0
002255:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
00225A:  74 4F                        je       0x22ab
00225C:  F6 06 34 25 01               test     byte ptr [0x2534], 1
002261:  75 48                        jne      0x22ab
002263:  F7 06 F3 24 04 00            test     word ptr [0x24f3], 4
002269:  75 40                        jne      0x22ab
00226B:  83 3E 88 67 08               cmp      word ptr [0x6788], 8
002270:  72 07                        jb       0x2279
002272:  83 3E 88 67 28               cmp      word ptr [0x6788], 0x28
002277:  76 32                        jbe      0x22ab
002279:  83 3E 88 67 04               cmp      word ptr [0x6788], 4
00227E:  0F 94 06 34 25               sete     byte ptr [0x2534]
002283:  66 A1 78 0D                  mov      eax, dword ptr [0xd78]
002287:  66 A3 84 0D                  mov      dword ptr [0xd84], eax
00228B:  66 A1 7C 0D                  mov      eax, dword ptr [0xd7c]
00228F:  66 A3 88 0D                  mov      dword ptr [0xd88], eax
002293:  9A 47 0A 71 09               lcall    0x971, 0xa47
002298:  BF 51 52                     mov      di, 0x5251
00229B:  B9 60 00                     mov      cx, 0x60
00229E:  66 33 C0                     xor      eax, eax
0022A1:  F3 66 AB                     rep stosd
0022A4:  C6 06 55 5B 01               mov      byte ptr [0x5b55], 1
0022A9:  EB 03                        jmp      0x22ae
0022AB:  E8 22 00                     call     0x22d0
0022AE:  59                           pop      cx
0022AF:  66 58                        pop      eax
0022B1:  C3                           ret
