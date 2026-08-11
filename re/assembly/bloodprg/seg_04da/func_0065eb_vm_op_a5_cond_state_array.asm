; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0065eb
; seg_off: 04da:124b
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a5_cond_state_array
; label_comment: VM opcode 0xA5: lodsb index; bp=index*2; gated on gs:[0x67ad]&1; test word gs:[bp+0x6ade] (the state_word_array) - conditional branch on a state-array flag
; incoming: vm_opcode_handlers:opcode_0xa5
; byte_count: 33
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x660b:1, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0065eb_vm_op_a5_cond_state_array.cpp
; routine_bytes_sha256: 185dcc8383a9d898b05fab9279e67ed099db5d7ca64dc174f3842cce4a57d2ed

0065EB:  AC                           lodsb    al, byte ptr [si]
0065EC:  98                           cwde    
0065ED:  03 C0                        add      ax, ax
0065EF:  8B E8                        mov      bp, ax
0065F1:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
0065F7:  74 0D                        je       0x6606
0065F9:  F7 86 DE 6A FF FF            test     word ptr [bp + 0x6ade], 0xffff
0065FF:  74 0A                        je       0x660b
006601:  E8 5E FE                     call     0x6462
006604:  EB 05                        jmp      0x660b
006606:  AD                           lodsw    ax, word ptr [si]
006607:  89 86 DE 6A                  mov      word ptr [bp + 0x6ade], ax
00660B:  C3                           ret     
