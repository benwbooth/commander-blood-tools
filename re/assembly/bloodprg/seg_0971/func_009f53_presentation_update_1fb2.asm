; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009f53
; seg_off: 0971:0243
; group: seg_0971
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: presentation_update_1fb2
; label_comment: presentation update step: test [0x1fb2]&1; call 0xa2dd; test [0x24f3]&8. A per-frame dialogue/presentation state-machine step gated on the [0x1fb2]/[0x24f3] flags
; incoming: call@0x0012db->0971:0243
; incoming: call@0x007bdd->0971:0243
; incoming: call@0x007bfd->0971:0243
; incoming: call@0x0081b1->0971:0243
; byte_count: 45
; boundary: cfg_blocks_5_terminals_1
; terminal: retf:1
; direct_callees: 0x00a2dd
; indirect_calls: 0
; routine_bytes_sha256: ced1a45f7a02ce1d83ab877ecee1b36eb4147bbae70f5294d32b7d860663bf3e

009F53:  50                           push     ax
009F54:  53                           push     bx
009F55:  51                           push     cx
009F56:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
009F5B:  74 1F                        je       0x9f7c
009F5D:  E8 7D 03                     call     0xa2dd
009F60:  F6 06 F3 24 08               test     byte ptr [0x24f3], 8
009F65:  74 05                        je       0x9f6c
009F67:  C6 06 D8 27 01               mov      byte ptr [0x27d8], 1
009F6C:  C7 06 88 67 FF FF            mov      word ptr [0x6788], 0xffff
009F72:  C6 06 B2 1F 00               mov      byte ptr [0x1fb2], 0
009F77:  80 26 AA 67 FD               and      byte ptr [0x67aa], 0xfd
009F7C:  59                           pop      cx
009F7D:  5B                           pop      bx
009F7E:  58                           pop      ax
009F7F:  CB                           retf    
