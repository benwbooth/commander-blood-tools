; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a73e
; seg_off: 0971:0a2e
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_bounds_init
; label_comment: init the gs:0xd8c list bounds (2 calls): [0xd60]=0, [0xd62]=0, [0xd64]=0xffff, [0xd66]=0xffff. Resets the list's min/max bound pointers (part of the EMS-banked list subsystem)
; byte_count: 25
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 0e917d5682932dee6f5d2a043ab17eb0e7c422840fa501810af051633ac3e21d

00A73E:  C7 06 60 0D 00 00            mov      word ptr [0xd60], 0
00A744:  C7 06 62 0D 00 00            mov      word ptr [0xd62], 0
00A74A:  C7 06 64 0D FF FF            mov      word ptr [0xd64], 0xffff
00A750:  C7 06 66 0D FF FF            mov      word ptr [0xd66], 0xffff
00A756:  C3                           ret
