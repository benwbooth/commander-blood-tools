; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006559
; seg_off: 04da:11b9
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a0_push
; label_comment: VM opcode 0xA0 PUSH: set [0x67ad]=1; bp=stack_ptr gs:[0x6884]; stack_ptr+=2; lodsw operand -> [bp+0x6820]. Pushes the 16-bit operand onto the VM stack (gs:0x6820, ptr gs:0x6884)
; incoming: vm_opcode_handlers:opcode_0xa0
; byte_count: 25
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: f50046678408b664ce607278ea5a0bd5f68dcf525a0cc360c1ffc5fb41f60ba0

006559:  65 C6 06 AD 67 01            mov      byte ptr gs:[0x67ad], 1
00655F:  65 A1 84 68                  mov      ax, word ptr gs:[0x6884]
006563:  8B E8                        mov      bp, ax
006565:  83 C0 02                     add      ax, 2
006568:  65 A3 84 68                  mov      word ptr gs:[0x6884], ax
00656C:  AD                           lodsw    ax, word ptr [si]
00656D:  89 86 20 68                  mov      word ptr [bp + 0x6820], ax
006571:  C3                           ret     
