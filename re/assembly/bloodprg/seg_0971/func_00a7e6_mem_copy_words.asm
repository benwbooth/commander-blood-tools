; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a7e6
; seg_off: 0971:0ad6
; group: seg_0971
; provenance: recursive_graph
; label: mem_copy_words
; label_comment: word-block copy (2 calls): rep-style movsw es:[di]<-[si]. Copies N words between buffers
; byte_count: 7
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a7e6_mem_copy_words.cpp
; routine_bytes_sha256: 6aa5c60d59aa4dd835e5df01e31aca24da6cd83fb35b6b96dfb1381bbf9de5b2

00A7E6:  1E                           push     ds
00A7E7:  07                           pop      es
00A7E8:  A5                           movsw    word ptr es:[di], word ptr [si]
00A7E9:  A5                           movsw    word ptr es:[di], word ptr [si]
00A7EA:  A5                           movsw    word ptr es:[di], word ptr [si]
00A7EB:  A5                           movsw    word ptr es:[di], word ptr [si]
00A7EC:  C3                           ret     
