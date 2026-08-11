; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0064ce
; seg_off: 04da:112e
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_cc_set_record_byte
; label_comment: VM opcode 0xCC: bp=0x6cde + (operand1-1)*16; [bp]=operand2. Writes a byte into a 16-byte-record table at gs:0x6cde indexed by (operand-1) - a set-property/set-variable op
; incoming: vm_opcode_handlers:opcode_0xcc
; byte_count: 23
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: fb49c1c8da8dd70c2bcb24142f510ca420315f53a75896f5798bc6b3d8126ab5

0064CE:  BD DE 6C                     mov      bp, 0x6cde
0064D1:  AC                           lodsb    al, byte ptr [si]
0064D2:  FE C8                        dec      al
0064D4:  98                           cwde    
0064D5:  C1 E0 04                     shl      ax, 4
0064D8:  03 E8                        add      bp, ax
0064DA:  AC                           lodsb    al, byte ptr [si]
0064DB:  88 46 00                     mov      byte ptr [bp], al
0064DE:  45                           inc      bp
0064DF:  0A C0                        or       al, al
0064E1:  75 F7                        jne      0x64da
0064E3:  46                           inc      si
0064E4:  C3                           ret     
