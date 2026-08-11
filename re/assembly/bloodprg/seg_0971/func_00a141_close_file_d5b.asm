; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a141
; seg_off: 0971:0431
; group: seg_0971
; provenance: recursive_graph
; label: close_file_d5b
; label_comment: file-close helper (3 calls): bx=[0xd5b]; if bx!=0 and bx!=[0xa86], clear [0xd5b]=0 and DOS-close (ah=0x3e, int21h) the handle. Closes the file handle held at [0xd5b] if open and not the reserved one
; byte_count: 30
; boundary: cfg_blocks_4_terminals_1
; terminal: ret:1
; direct_callees: 0x00a73e
; indirect_calls: 0
; routine_bytes_sha256: 04deb165f2b81e49c1debc75c0a6481d6b5910c315a03dfef2da7634c8f8e43f

00A141:  8B 1E 5B 0D                  mov      bx, word ptr [0xd5b]
00A145:  0B DB                        or       bx, bx
00A147:  74 13                        je       0xa15c
00A149:  3B 1E 86 0A                  cmp      bx, word ptr [0xa86]
00A14D:  74 0D                        je       0xa15c
00A14F:  C7 06 5B 0D 00 00            mov      word ptr [0xd5b], 0
00A155:  B4 3E                        mov      ah, 0x3e
00A157:  CD 21                        int      0x21
00A159:  E8 E2 05                     call     0xa73e
00A15C:  33 C9                        xor      cx, cx
00A15E:  C3                           ret     
