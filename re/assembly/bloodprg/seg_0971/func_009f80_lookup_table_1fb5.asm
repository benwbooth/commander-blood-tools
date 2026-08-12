; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009f80
; seg_off: 0971:0270
; group: seg_0971
; provenance: recursive_graph
; label: lookup_table_1fb5
; label_comment: resource-descriptor index lookup (5 calls): AX selects a 4-byte DS:0x1fb5 entry and BX receives its first word, a near pointer to a {flags,variant,filename} resource descriptor. resource_switch 0x009f8e writes descriptor byte +1, loads the flags word at +0, and passes +2 as the filename. The four ADDs preserve AX and wrap the table offset to 16 bits.
; byte_count: 14
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 2e3eabf98179886172a5201127a38719a696bdb5eec9828c91d10a9b422ae85a

009F80:  BB B5 1F                     mov      bx, 0x1fb5
009F83:  03 D8                        add      bx, ax
009F85:  03 D8                        add      bx, ax
009F87:  03 D8                        add      bx, ax
009F89:  03 D8                        add      bx, ax
009F8B:  8B 1F                        mov      bx, word ptr [bx]
009F8D:  C3                           ret     
