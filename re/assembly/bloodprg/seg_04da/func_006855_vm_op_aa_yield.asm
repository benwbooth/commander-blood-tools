; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006855
; seg_off: 04da:14b5
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_aa_yield
; label_comment: VM opcode 0xAA: set gs:[0x67b4]=1 - the yield flag the exec loop checks after each handler (0x562a); makes the VM break/yield the current frame. A yield/wait opcode
; incoming: vm_opcode_handlers:opcode_0xaa
; byte_count: 7
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 93213edc4d025c4676206e12c9c85b2e17447e1c703690da9ae2c2c4d7bd1637

006855:  65 C6 06 B4 67 01            mov      byte ptr gs:[0x67b4], 1
00685B:  C3                           ret     
