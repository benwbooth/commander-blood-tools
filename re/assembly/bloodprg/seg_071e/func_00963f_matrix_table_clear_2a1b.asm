; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00963f
; seg_off: 071e:1e5f
; group: seg_071e
; provenance: recursive_graph
; label: matrix_table_clear_2a1b
; label_comment: clear the 3D matrix/object table: bp=0x2a1b; cx=6 records; [bp]=0; bp+=0x18 (24-byte stride). Zeros 6x 24-byte records (the per-object transform slots at 0x2a1b)
; byte_count: 23
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 60225baa9b9f1b75e86b7849f4a7b8b9dff1baf628d87ec419d1ed2e67568a32

00963F:  50                           push     ax
009640:  51                           push     cx
009641:  55                           push     bp
009642:  BD 1B 2A                     mov      bp, 0x2a1b
009645:  B9 06 00                     mov      cx, 6
009648:  C7 46 00 00 00               mov      word ptr [bp], 0
00964D:  83 C5 18                     add      bp, 0x18
009650:  E2 F6                        loop     0x9648
009652:  5D                           pop      bp
009653:  59                           pop      cx
009654:  58                           pop      ax
009655:  CB                           retf    
