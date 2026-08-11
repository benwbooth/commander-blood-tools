; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005fd8
; seg_off: 04da:0c38
; group: seg_04da
; provenance: recursive_graph
; label: vm_special_slot_remove
; label_comment: remove AX from 16-word sentinel list DS:0x6d3e; CF set on hit
; byte_count: 30
; boundary: cfg_blocks_4_terminals_2
; terminal: jmp 0x5ff3:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: cad4df5d5c6daadda7971bd678e328bcd53205e1b2ae34dd698c9c9656510fe2

005FD8:  51                           push     cx
005FD9:  55                           push     bp
005FDA:  BD 3E 6D                     mov      bp, 0x6d3e
005FDD:  B9 10 00                     mov      cx, 0x10
005FE0:  3B 46 00                     cmp      ax, word ptr [bp]
005FE3:  74 08                        je       0x5fed
005FE5:  83 C5 02                     add      bp, 2
005FE8:  E2 F6                        loop     0x5fe0
005FEA:  F8                           clc     
005FEB:  EB 06                        jmp      0x5ff3
005FED:  C7 46 00 00 00               mov      word ptr [bp], 0
005FF2:  F9                           stc     
005FF3:  5D                           pop      bp
005FF4:  59                           pop      cx
005FF5:  C3                           ret     
