; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a642
; seg_off: 0971:0932
; group: seg_0971
; provenance: recursive_graph
; label: banked_list_load
; label_comment: load the gs:0xd8c banked list: call list_d8c_init 0xa757; call list_d8c_read 0xa622; di=[0x5233] (buffer end). Fills the EMS-banked ring buffer from the resource
; byte_count: 252
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x00a622, 0x00a757
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a642_banked_list_load.cpp
; routine_bytes_sha256: ad608c6fb80044aeec02c44132c4bd973b66c4c1e6431349799532debce68133

00A642:  0E                           push     cs
00A643:  E8 11 01                     call     0xa757
00A646:  E8 D9 FF                     call     0xa622
00A649:  0F 82 F0 00                  jb       0xa73d
00A64D:  8B 3E 33 52                  mov      di, word ptr [0x5233]
00A651:  2B F8                        sub      di, ax
00A653:  83 EF 02                     sub      di, 2
00A656:  89 3E 90 0D                  mov      word ptr [0xd90], di
00A65A:  AB                           stosw    word ptr es:[di], ax
00A65B:  89 3E 8C 0D                  mov      word ptr [0xd8c], di
00A65F:  8B C8                        mov      cx, ax
00A661:  83 E9 02                     sub      cx, 2
; -- non-contiguous block: next 0x00a73d --
00A73D:  C3                           ret     
