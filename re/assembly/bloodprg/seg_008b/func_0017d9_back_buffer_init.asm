; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0017d9
; seg_off: 008b:0929
; group: seg_008b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: back_buffer_init
; label_comment: back-buffer setup: si=0xea; les di,[0x5229] (linear back-buffer); clear [0x5b53]=0, [0x5b57]=0 (render-ready flags). Initializes the linear composition buffer + its state flags
; incoming: call@0x00b555->008b:0929
; incoming: call@0x00b675->008b:0929
; byte_count: 62
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 2
; routine_bytes_sha256: 187df516a277a3c3d50715b93758e2c0c8eec83294473e1f1577596190c4b428

0017D9:  1E                           push     ds
0017DA:  56                           push     si
0017DB:  06                           push     es
0017DC:  57                           push     di
0017DD:  BE EA 00                     mov      si, 0xea
0017E0:  C4 3E 29 52                  les      di, ptr [0x5229]
0017E4:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
0017E9:  C6 06 57 5B 00               mov      byte ptr [0x5b57], 0
0017EE:  9A 1D 09 CE 01               lcall    0x1ce, 0x91d
0017F3:  C5 36 29 52                  lds      si, ptr [0x5229]
0017F7:  66 65 FF 36 19 52            push     dword ptr gs:[0x5219]
0017FD:  66 65 C7 06 19 52 00 C0 00 A0 mov      dword ptr gs:[0x5219], 0xa000c000
001807:  9A 3E 0F 99 02               lcall    0x299, 0xf3e
00180C:  66 65 8F 06 19 52            pop      dword ptr gs:[0x5219]
001812:  5F                           pop      di
001813:  07                           pop      es
001814:  5E                           pop      si
001815:  1F                           pop      ds
001816:  CB                           retf    
