; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0064c0
; seg_off: 04da:1120
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_cf_clear_state
; label_comment: VM opcode 0xCF: clear gs:[0x67b1]=0 and gs:[0x6764]=0 (reset resume/PC-related VM state)
; incoming: vm_opcode_handlers:opcode_0xcf
; byte_count: 14
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0064c0_vm_op_cf_clear_state.cpp
; routine_bytes_sha256: 5883e6dc39e258e62b23a50005b0c89a3788ce559ac50e8e421298dd941e2bbf

0064C0:  65 C6 06 B1 67 00            mov      byte ptr gs:[0x67b1], 0
0064C6:  65 C7 06 64 67 00 00         mov      word ptr gs:[0x6764], 0
0064CD:  C3                           ret     
