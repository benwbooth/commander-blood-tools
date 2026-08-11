; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009f8e
; seg_off: 0971:027e
; group: seg_0971
; provenance: recursive_graph
; label: resource_switch
; label_comment: resource switch/reload (2 calls): [0xd80]=ax (new id); call close_file_d5b 0xa141 (close current); push cs; call list_d8c_init 0xa757 (reinit the banked list). Switches the active banked resource - close old file + reset the list
; byte_count: 20
; boundary: cfg_blocks_1_terminals_0
; terminal: none
; direct_callees: 0x00a141, 0x00a73e, 0x00a757
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_009f8e_resource_switch.cpp
; routine_bytes_sha256: 579c18105617c7ee134a7cea71849bb5dd874484c4e9ff775f994a0908cc52dc

009F8E:  66 50                        push     eax
009F90:  A3 80 0D                     mov      word ptr [0xd80], ax
009F93:  E8 AB 01                     call     0xa141
009F96:  0E                           push     cs
009F97:  E8 BD 07                     call     0xa757
009F9A:  C6 06 5F 0D 00               mov      byte ptr [0xd5f], 0
009F9F:  E8 9C 07                     call     0xa73e
