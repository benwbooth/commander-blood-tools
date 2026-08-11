; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005ff6
; seg_off: 04da:0c56
; group: seg_04da
; provenance: recursive_graph
; label: vm_special_slot_insert
; label_comment: insert AX into 16-word sentinel list DS:0x6d3e; CF clear only when full
; byte_count: 45
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x6020:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_005ff6_vm_special_slot_insert.cpp
; routine_bytes_sha256: ff7b1ecd0e7fe8001d9779a9e5323bfceffc67deb4f66b81891bfe1b36226165

005FF6:  51                           push     cx
005FF7:  55                           push     bp
005FF8:  BD 3E 6D                     mov      bp, 0x6d3e
005FFB:  B9 10 00                     mov      cx, 0x10
005FFE:  3B 46 00                     cmp      ax, word ptr [bp]
006001:  74 1C                        je       0x601f
006003:  83 C5 02                     add      bp, 2
006006:  E2 F6                        loop     0x5ffe
006008:  BD 3E 6D                     mov      bp, 0x6d3e
00600B:  B9 10 00                     mov      cx, 0x10
00600E:  83 7E 00 00                  cmp      word ptr [bp], 0
006012:  74 08                        je       0x601c
006014:  83 C5 02                     add      bp, 2
006017:  E2 F5                        loop     0x600e
006019:  F8                           clc     
00601A:  EB 04                        jmp      0x6020
00601C:  89 46 00                     mov      word ptr [bp], ax
00601F:  F9                           stc     
006020:  5D                           pop      bp
006021:  59                           pop      cx
006022:  C3                           ret     
