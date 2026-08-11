; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008c96
; seg_off: 071e:14b6
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vm_segment_call_wrapper
; label_comment: wrapper (4 calls): saves regs, lcall 0x4da:0x1c53 (into the VM code segment), post-processes with gs. A thunk into a VM-segment routine
; incoming: call@0x0010eb->071e:14b6
; incoming: call@0x001a89->071e:14b6
; incoming: call@0x001d48->071e:14b6
; incoming: call@0x005e94->071e:14b6
; incoming: call@0x00b55a->071e:14b6
; byte_count: 56
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_071e/func_008c96_vm_segment_call_wrapper.cpp
; routine_bytes_sha256: f42a501f52c61d36a9b2deb444f2743502a69ae48ed908599ab15756d67212e6

008C96:  55                           push     bp
008C97:  50                           push     ax
008C98:  1E                           push     ds
008C99:  06                           push     es
008C9A:  57                           push     di
008C9B:  56                           push     si
008C9C:  51                           push     cx
008C9D:  9A 53 1C DA 04               lcall    0x4da, 0x1c53
008CA2:  8C E8                        mov      ax, gs
008CA4:  8E D8                        mov      ds, ax
008CA6:  8E C0                        mov      es, ax
008CA8:  BE D1 53                     mov      si, 0x53d1
008CAB:  BF D8 5C                     mov      di, 0x5cd8
008CAE:  B9 30 00                     mov      cx, 0x30
008CB1:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
008CB4:  C7 06 65 2F 10 27            mov      word ptr [0x2f65], 0x2710
008CBA:  C7 06 67 2F E0 2E            mov      word ptr [0x2f67], 0x2ee0
008CC0:  C7 06 69 2F 00 00            mov      word ptr [0x2f69], 0
008CC6:  59                           pop      cx
008CC7:  5E                           pop      si
008CC8:  5F                           pop      di
008CC9:  07                           pop      es
008CCA:  1F                           pop      ds
008CCB:  58                           pop      ax
008CCC:  5D                           pop      bp
008CCD:  CB                           retf    
