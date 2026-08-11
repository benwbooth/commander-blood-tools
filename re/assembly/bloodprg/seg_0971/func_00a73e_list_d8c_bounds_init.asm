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
; byte_count: 6
; boundary: cfg_blocks_1_terminals_0
; terminal: none
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 88b5d36ec038b13cc13ba402d872646d1cdb71508e3b78cde404af933938b546

00A73E:  C7 06 60 0D 00 00            mov      word ptr [0xd60], 0
