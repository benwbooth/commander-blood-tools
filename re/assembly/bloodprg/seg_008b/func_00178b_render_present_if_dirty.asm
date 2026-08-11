; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00178b
; seg_off: 008b:08db
; group: seg_008b
; provenance: recursive_graph
; label: render_present_if_dirty
; label_comment: conditional screen present (3 calls): if [0x5b55]&1 (dirty), lcall 0:0x5d7 + si=0x5251 + lcall 0x299:0 (present/blit), then clear [0x5b55]=0 and [0xa40]=0. Flushes the composed frame to the display only when dirty
; byte_count: 36
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_008b/func_00178b_render_present_if_dirty.cpp
; routine_bytes_sha256: 56810a11ff589407d2745bfac0ab53dc5b527318438fbea78feb8e6b59bff08b

00178B:  F6 06 55 5B 01               test     byte ptr [0x5b55], 1
001790:  74 1C                        je       0x17ae
001792:  9A D7 05 00 00               lcall    0, 0x5d7
001797:  BE 51 52                     mov      si, 0x5251
00179A:  9A 00 00 99 02               lcall    0x299, 0
00179F:  C6 06 55 5B 00               mov      byte ptr [0x5b55], 0
0017A4:  C6 06 40 0A 00               mov      byte ptr [0xa40], 0
0017A9:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
0017AE:  C3                           ret     
