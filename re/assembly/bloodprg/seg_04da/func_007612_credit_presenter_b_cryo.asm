; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007612
; seg_off: 04da:2272
; group: seg_04da
; provenance: static_dispatch_table_target
; label: credit_presenter_b_cryo
; label_comment: CRYO credit presenter: copies SI->gs:0xe18, sets 5e64=1 5e58=0 (arms clean reveal). NEVER dispatched (linear 0x8a32 confirmed via exec_watch_linear)
; incoming: byte_parser_dispatch_74e5:byte_0x05
; byte_count: 23
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 7e718bce20f875afc5bbc230ab874e536d9f4156a6c8ab73370d934a8d318449

007612:  BF 18 0E                     mov      di, 0xe18
007615:  AC                           lodsb    al, byte ptr [si]
007616:  AA                           stosb    byte ptr es:[di], al
007617:  0A C0                        or       al, al
007619:  75 FA                        jne      0x7615
00761B:  65 C6 06 64 5E 01            mov      byte ptr gs:[0x5e64], 1
007621:  65 C7 06 58 5E 00 00         mov      word ptr gs:[0x5e58], 0
007628:  C3                           ret     
