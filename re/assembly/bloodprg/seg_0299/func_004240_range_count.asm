; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x004240
; seg_off: 0299:12b0
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: range_count
; label_comment: count/length calc: cx = bx - ax + 1 (inclusive range length). A span-length helper
; incoming: call@0x008ad4->0299:12b0
; byte_count: 45
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: a7ed64c1f53178c137384872786bbb9eb520553a7829fe72853109873ee69eb3

004240:  50                           push     ax
004241:  53                           push     bx
004242:  51                           push     cx
004243:  1E                           push     ds
004244:  56                           push     si
004245:  8B CB                        mov      cx, bx
004247:  2B C8                        sub      cx, ax
004249:  41                           inc      cx
00424A:  8C EB                        mov      bx, gs
00424C:  8E DB                        mov      ds, bx
00424E:  BE 12 62                     mov      si, 0x6212
004251:  C1 E0 05                     shl      ax, 5
004254:  03 F0                        add      si, ax
004256:  8B 04                        mov      ax, word ptr [si]
004258:  0A C0                        or       al, al
00425A:  79 06                        jns      0x4262
00425C:  24 7E                        and      al, 0x7e
00425E:  0C 02                        or       al, 2
004260:  89 04                        mov      word ptr [si], ax
004262:  83 C6 20                     add      si, 0x20
004265:  E2 EF                        loop     0x4256
004267:  5E                           pop      si
004268:  1F                           pop      ds
004269:  59                           pop      cx
00426A:  5B                           pop      bx
00426B:  58                           pop      ax
00426C:  CB                           retf    
