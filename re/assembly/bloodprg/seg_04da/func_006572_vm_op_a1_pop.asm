; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006572
; seg_off: 04da:11d2
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a1_pop
; label_comment: VM opcode 0xA1 POP: set [0x67ad]=0; if stack_ptr gs:[0x6884]==2 (empty) noop else stack_ptr-=2. Pops the VM stack
; incoming: vm_opcode_handlers:opcode_0xa1
; byte_count: 22
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: bc6581c1f7c99b194d632b47ae66b97bed7177f82acc5b4f84e7cac0ef0beeac

006572:  65 C6 06 AD 67 00            mov      byte ptr gs:[0x67ad], 0
006578:  65 A1 84 68                  mov      ax, word ptr gs:[0x6884]
00657C:  83 F8 02                     cmp      ax, 2
00657F:  74 06                        je       0x6587
006581:  65 83 2E 84 68 02            sub      word ptr gs:[0x6884], 2
006587:  C3                           ret     
