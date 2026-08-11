; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001fbc
; seg_off: 008b:110c
; group: seg_008b
; provenance: recursive_graph
; label: flag_test_a2e
; label_comment: flag test: ax=[0xa2e]; test al,1; branch. Reads the 0xa2e state word and dispatches on its low bit
; byte_count: 50
; boundary: cfg_blocks_7_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 223c44bf3248ca556f9f5740f65fae46529a1ca556d5a5f3e0a4fa84a99a0ea5

001FBC:  A1 2E 0A                     mov      ax, word ptr [0xa2e]
001FBF:  A8 01                        test     al, 1
001FC1:  74 10                        je       0x1fd3
001FC3:  22 06 30 0A                  and      al, byte ptr [0xa30]
001FC7:  75 0A                        jne      0x1fd3
001FC9:  C6 06 3E 0A 01               mov      byte ptr [0xa3e], 1
001FCE:  C6 06 40 0A 01               mov      byte ptr [0xa40], 1
001FD3:  A8 02                        test     al, 2
001FD5:  74 10                        je       0x1fe7
001FD7:  22 06 30 0A                  and      al, byte ptr [0xa30]
001FDB:  75 0A                        jne      0x1fe7
001FDD:  C6 06 3F 0A 01               mov      byte ptr [0xa3f], 1
001FE2:  C6 06 40 0A 01               mov      byte ptr [0xa40], 1
001FE7:  A1 2E 0A                     mov      ax, word ptr [0xa2e]
001FEA:  A3 30 0A                     mov      word ptr [0xa30], ax
001FED:  C3                           ret     
