; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00267d
; seg_off: 01ce:039d
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: kbd_read_int16
; label_comment: keyboard read: ax=0x100; int 16h (AH=01 check-keystroke); if none skip; else xor ax,ax; int 16h (AH=00 get-keystroke). BIOS keyboard poll+read
; incoming: call@0x002116->01ce:039d
; byte_count: 16
; boundary: cfg_blocks_4_terminals_2
; terminal: jmp 0x268c:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 21e0f726cfb344e30aebf093f112ed735321d07c46214f8d194875618280f831

00267D:  B8 00 01                     mov      ax, 0x100
002680:  CD 16                        int      0x16
002682:  74 06                        je       0x268a
002684:  33 C0                        xor      ax, ax
002686:  CD 16                        int      0x16
002688:  EB 02                        jmp      0x268c
00268A:  33 C0                        xor      ax, ax
00268C:  CB                           retf    
