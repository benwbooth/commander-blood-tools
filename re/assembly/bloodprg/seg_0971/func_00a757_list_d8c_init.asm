; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a757
; seg_off: 0971:0a47
; group: seg_0971
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: list_d8c_init
; label_comment: init of the gs:0xd8c list subsystem (3 calls): [0xd8e]=[0xa7e]; [0xd92]=[0xa7e] (reset head/tail pointers to the base [0xa7e]); zero followups. Resets the 0xd8c buffer/list
; incoming: call@0x002293->0971:0a47
; byte_count: 33
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a757_list_d8c_init.cpp
; routine_bytes_sha256: 1898e092b06c16b06473284e2f625a6c4aa576718d90f4c44fe81021ad09040f

00A757:  A1 7E 0A                     mov      ax, word ptr [0xa7e]
00A75A:  A3 8E 0D                     mov      word ptr [0xd8e], ax
00A75D:  A3 92 0D                     mov      word ptr [0xd92], ax
00A760:  33 C0                        xor      ax, ax
00A762:  A3 8C 0D                     mov      word ptr [0xd8c], ax
00A765:  A3 90 0D                     mov      word ptr [0xd90], ax
00A768:  A3 9A 0D                     mov      word ptr [0xd9a], ax
00A76B:  A3 A0 0D                     mov      word ptr [0xda0], ax
00A76E:  A3 96 0D                     mov      word ptr [0xd96], ax
00A771:  A1 33 52                     mov      ax, word ptr [0x5233]
00A774:  A3 98 0D                     mov      word ptr [0xd98], ax
00A777:  CB                           retf    
