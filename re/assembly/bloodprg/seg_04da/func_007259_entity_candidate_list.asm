; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007259
; seg_off: 04da:1eb9
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: entity_candidate_list
; label_comment: builds the PRESENTABLE-OBJECT candidate list: source list @gs:0x6886 (0x624B nav-source build), filtered by flags&0x98 + rec[+2]&2, survivors written to [0x250B]++ — the world/entity selection is LIST-DRIVEN (choice-box rows = candidates; commit picks via +0x16 linkage). NO free-roam mouse hover hit-test exists — consistent with the universal choice-box console model. Port adjustment: on-planet interaction = entity list box, not free clicks || NARROWER EARLIER READING `vm_text_helper_6886`: VM/dialogue helper: ds=ax; bp=0x6886; call 0x624b. Processes dialogue/text state at gs:0x6886 (near the active-line record) || MERGED 2026-07-25 (audit-fixes #133): one address, two names, the shorter describing a prologue or a single facet. Kept because a narrow reading records a true observation; renamed away because it is not what the routine IS.
; incoming: call@0x00b0ee->04da:1eb9
; incoming: call@0x00b105->04da:1eb9
; byte_count: 79
; boundary: cfg_blocks_9_terminals_3
; terminal: jmp 0x7273:1, jmp 0x727b:1, retf:1
; direct_callees: 0x00624b
; indirect_calls: 0
; routine_bytes_sha256: 09f6be86eda4305af0983636fe5efa9e81bf5037080b272f2b75f96fa2904e24

007259:  1E                           push     ds
00725A:  56                           push     si
00725B:  53                           push     bx
00725C:  50                           push     ax
00725D:  57                           push     di
00725E:  8C E8                        mov      ax, gs
007260:  8E D8                        mov      ds, ax
007262:  BD 86 68                     mov      bp, 0x6886
007265:  0E                           push     cs
007266:  E8 E2 EF                     call     0x624b
007269:  BE 86 68                     mov      si, 0x6886
00726C:  BD 0B 25                     mov      bp, 0x250b
00726F:  8B C7                        mov      ax, di
007271:  EB 08                        jmp      0x727b
007273:  AD                           lodsw    ax, word ptr [si]
007274:  83 F8 FF                     cmp      ax, -1
007277:  74 24                        je       0x729d
007279:  8B F8                        mov      di, ax
00727B:  26 8B 1D                     mov      bx, word ptr es:[di]
00727E:  F7 C3 98 00                  test     bx, 0x98
007282:  74 17                        je       0x729b
007284:  26 F6 45 02 02               test     byte ptr es:[di + 2], 2
007289:  74 10                        je       0x729b
00728B:  65 3B 3E 52 67               cmp      di, word ptr gs:[0x6752]
007290:  74 09                        je       0x729b
007292:  83 C0 04                     add      ax, 4
007295:  89 46 00                     mov      word ptr [bp], ax
007298:  83 C5 02                     add      bp, 2
00729B:  EB D6                        jmp      0x7273
00729D:  C7 46 00 FF FF               mov      word ptr [bp], 0xffff
0072A2:  5F                           pop      di
0072A3:  58                           pop      ax
0072A4:  5B                           pop      bx
0072A5:  5E                           pop      si
0072A6:  1F                           pop      ds
0072A7:  CB                           retf    
