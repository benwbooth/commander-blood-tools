; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0022b2
; seg_off: 008b:1402
; group: seg_008b
; provenance: input_action_handler_table_index_15
; label: input_action_toggle_pause
; label_comment: Toggles pause for P/p unless the save UI is active, then always latches the raw key.
; byte_count: 30
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: 0x0022d0
; indirect_calls: 0
; routine_bytes_sha256: 3204f91cc26b18fbce4eed5a97e53e9474e64811520bf139dcaba701a713797c

0022B2:  F6 06 36 27 01               test     byte ptr [0x2736], 1
0022B7:  75 13                        jne      0x22cc
0022B9:  F6 06 DF 0A 01               test     byte ptr [0xadf], 1
0022BE:  75 07                        jne      0x22c7
0022C0:  C6 06 DF 0A 01               mov      byte ptr [0xadf], 1
0022C5:  EB 05                        jmp      0x22cc
0022C7:  C6 06 DF 0A 00               mov      byte ptr [0xadf], 0
0022CC:  E8 01 00                     call     0x22d0
0022CF:  C3                           ret
