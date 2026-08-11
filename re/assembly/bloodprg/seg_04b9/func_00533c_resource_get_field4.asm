; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00533c
; seg_off: 04b9:01ac
; group: seg_04b9
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_get_field4
; label_comment: resource-table +4 getter: bx=handle<<3; return eax = fs:[bx+4] (the dword at +4 of the entry - resource size or data offset)
; incoming: call@0x001c7f->04b9:01ac
; incoming: call@0x001d21->04b9:01ac
; byte_count: 13
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: f754ed5eb80294244f426d5a86fb7cf995f94729fba0a77a9152be814d0dc6d0

00533C:  53                           push     bx
00533D:  C1 E0 03                     shl      ax, 3
005340:  8B D8                        mov      bx, ax
005342:  66 64 8B 47 04               mov      eax, dword ptr fs:[bx + 4]
005347:  5B                           pop      bx
005348:  CB                           retf    
