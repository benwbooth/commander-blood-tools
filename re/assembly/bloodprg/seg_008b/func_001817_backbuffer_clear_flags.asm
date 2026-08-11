; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001817
; seg_off: 008b:0967
; group: seg_008b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: backbuffer_clear_flags
; label_comment: back-buffer setup: si=0xe3; les di,[0x5229] (the linear render back-buffer); clears byte [0x5b53] and [0x5b57]. Resets the two render-state flags before drawing into the back buffer
; incoming: call@0x00b0a6->008b:0967
; incoming: call@0x00b663->008b:0967
; byte_count: 62
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 2
; routine_bytes_sha256: 2b151a4f13e91f729130c124f06634dc898d53789c5477c0e8a8f019793160fe

001817:  1E                           push     ds
001818:  56                           push     si
001819:  06                           push     es
00181A:  57                           push     di
00181B:  BE E3 00                     mov      si, 0xe3
00181E:  C4 3E 29 52                  les      di, ptr [0x5229]
001822:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
001827:  C6 06 57 5B 00               mov      byte ptr [0x5b57], 0
00182C:  9A 1D 09 CE 01               lcall    0x1ce, 0x91d
001831:  C5 36 29 52                  lds      si, ptr [0x5229]
001835:  66 65 FF 36 19 52            push     dword ptr gs:[0x5219]
00183B:  66 65 C7 06 19 52 00 C0 00 A0 mov      dword ptr gs:[0x5219], 0xa000c000
001845:  9A 3E 0F 99 02               lcall    0x299, 0xf3e
00184A:  66 65 8F 06 19 52            pop      dword ptr gs:[0x5219]
001850:  5F                           pop      di
001851:  07                           pop      es
001852:  5E                           pop      si
001853:  1F                           pop      ds
001854:  CB                           retf    
