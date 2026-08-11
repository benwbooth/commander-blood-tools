; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006946
; seg_off: 04da:15a6
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_shared_record_wildcard
; label_comment: VM shared handler for 0xAD/0xAF/0xB2/0xB3/0xBA/0xBB/0xBC (7 opcodes, same op): dual-mode record op on gs:0x6724[bx+di], gated on the query flag gs:[0x67ad]. QUERY mode: lodsw bx offset + lodsw value; WILDCARD - if value==gs:[0x674e] substitute 0xffff (match-any); cmp vs es:[bx+di], branch via 0x6462 on mismatch. SET mode: write path (0x6985). These 7 opcode values all execute this same generic record compare/write with wildcard - likely script-readability verbs mapping to one op || ALSO RECORDED as `vm_op_shared_state_gated`: VM opcode SHARED handler for 0xAD/0xAF/0xB2/0xB3/0xBA/0xBB/0xBC (7 opcodes -> same handler = same operation): les di,gs:[0x6724] state table; gated on gs:[0x67ad]&1; 0xA1-skip; lodsw bx record offset/value. A state-table record op these opcodes all perform identically (aliased/grouped opcodes). Exact field op partially decoded (prologue + record access confirmed) || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xad
; incoming: vm_opcode_handlers:opcode_0xaf
; incoming: vm_opcode_handlers:opcode_0xb2
; incoming: vm_opcode_handlers:opcode_0xb3
; incoming: vm_opcode_handlers:opcode_0xba
; incoming: vm_opcode_handlers:opcode_0xbb
; incoming: vm_opcode_handlers:opcode_0xbc
; byte_count: 129
; boundary: cfg_blocks_20_terminals_4
; terminal: jmp 0x69c2:1, jmp 0x69c5:2, ret:1
; direct_callees: 0x005fd8, 0x005ff6, 0x006034, 0x006462
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_006946_vm_op_shared_record_wildcard.cpp
; routine_bytes_sha256: ddff05a758bb8e61b5d1c3e6256399a6ba777b756fdacbfa09dd7111343b3fd3

006946:  57                           push     di
006947:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
00694C:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006952:  74 31                        je       0x6985
006954:  32 D2                        xor      dl, dl
006956:  8A 34                        mov      dh, byte ptr [si]
006958:  80 FE A1                     cmp      dh, 0xa1
00695B:  75 03                        jne      0x6960
00695D:  FE C2                        inc      dl
00695F:  46                           inc      si
006960:  AD                           lodsw    ax, word ptr [si]
006961:  8B D8                        mov      bx, ax
006963:  AD                           lodsw    ax, word ptr [si]
006964:  65 3B 06 4E 67               cmp      ax, word ptr gs:[0x674e]
006969:  75 03                        jne      0x696e
00696B:  B8 FF FF                     mov      ax, 0xffff
00696E:  26 3B 01                     cmp      ax, word ptr es:[bx + di]
006971:  75 09                        jne      0x697c
006973:  0A D2                        or       dl, dl
006975:  74 4E                        je       0x69c5
006977:  E8 E8 FA                     call     0x6462
00697A:  EB 49                        jmp      0x69c5
00697C:  0A D2                        or       dl, dl
00697E:  75 45                        jne      0x69c5
006980:  E8 DF FA                     call     0x6462
006983:  EB 40                        jmp      0x69c5
006985:  AD                           lodsw    ax, word ptr [si]
006986:  8B D8                        mov      bx, ax
006988:  AD                           lodsw    ax, word ptr [si]
006989:  8A 4C FB                     mov      cl, byte ptr [si - 5]
00698C:  80 F9 BC                     cmp      cl, 0xbc
00698F:  75 04                        jne      0x6995
006991:  65 A3 82 67                  mov      word ptr gs:[0x6782], ax
006995:  26 83 39 FF                  cmp      word ptr es:[bx + di], -1
006999:  75 0E                        jne      0x69a9
00699B:  50                           push     ax
00699C:  53                           push     bx
00699D:  8B C3                        mov      ax, bx
00699F:  E8 92 F6                     call     0x6034
0069A2:  E8 33 F6                     call     0x5fd8
0069A5:  5B                           pop      bx
0069A6:  58                           pop      ax
0069A7:  EB 19                        jmp      0x69c2
0069A9:  65 3B 06 4E 67               cmp      ax, word ptr gs:[0x674e]
0069AE:  74 05                        je       0x69b5
0069B0:  83 F8 FF                     cmp      ax, -1
0069B3:  75 0D                        jne      0x69c2
0069B5:  8B C3                        mov      ax, bx
0069B7:  E8 7A F6                     call     0x6034
0069BA:  E8 39 F6                     call     0x5ff6
0069BD:  73 06                        jae      0x69c5
0069BF:  B8 FF FF                     mov      ax, 0xffff
0069C2:  26 89 01                     mov      word ptr es:[bx + di], ax
0069C5:  5F                           pop      di
0069C6:  C3                           ret     
