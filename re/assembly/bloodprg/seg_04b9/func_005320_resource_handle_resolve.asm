; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005320
; seg_off: 04b9:0190
; group: seg_04b9
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_handle_resolve
; label_comment: SEG 0x4b9:0x190: resolve resource handle AX->loaded segment. bx=AX<<3 (8-byte entries); if fs:[bx+2]&3 set, ds=fs:[bx]=resource segment, si=0, ax=1(loaded) else ax=0. Resource handle table = fs base, {seg u16, flags u16, ...} per 8-byte entry
; incoming: call@0x000fa1->04b9:0190
; incoming: call@0x0040e1->04b9:0190
; incoming: call@0x005476->04b9:0190
; incoming: call@0x005481->04b9:0190
; incoming: call@0x0055e5->04b9:0190
; byte_count: 28
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 5f1da61abdc40d59f1100082755e9b954b3c1ca6bb7b4830b43eb85ed30f83f1

005320:  53                           push     bx
005321:  C1 E0 03                     shl      ax, 3
005324:  8B D8                        mov      bx, ax
005326:  33 C0                        xor      ax, ax
005328:  64 F7 47 02 03 00            test     word ptr fs:[bx + 2], 3
00532E:  74 0A                        je       0x533a
005330:  64 8B 07                     mov      ax, word ptr fs:[bx]
005333:  8E D8                        mov      ds, ax
005335:  33 F6                        xor      si, si
005337:  B8 01 00                     mov      ax, 1
00533A:  5B                           pop      bx
00533B:  CB                           retf    
