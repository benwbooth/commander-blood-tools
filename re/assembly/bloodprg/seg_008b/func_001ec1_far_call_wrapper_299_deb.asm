; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001ec1
; seg_off: 008b:1011
; group: seg_008b
; provenance: recursive_graph
; label: far_call_wrapper_299_deb
; label_comment: far-call wrapper: xor ax,ax; lcall 0x299:0xdeb; xor ax,ax. Thin wrapper invoking the 0x299:0xdeb far routine with a zero argument
; byte_count: 79
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x1ed2:1, ret:1
; direct_callees: 0x00178b, 0x0017af, 0x00210e
; indirect_calls: 4
; cxx_source: re/borland/bloodprg/seg_008b/func_001ec1_far_call_wrapper_299_deb.cpp
; routine_bytes_sha256: 03d13c948ca0c97cc97cc8b7d6f970ded618a7f10e97ba1d70caec6370633334

001EC1:  33 C0                        xor      ax, ax
001EC3:  9A EB 0D 99 02               lcall    0x299, 0xdeb
001EC8:  33 C0                        xor      ax, ax
001ECA:  9A 2F 0E 99 02               lcall    0x299, 0xe2f
001ECF:  A3 88 67                     mov      word ptr [0x6788], ax
001ED2:  0E                           push     cs
001ED3:  E8 38 02                     call     0x210e
001ED6:  F6 06 13 0B 01               test     byte ptr [0xb13], 1
001EDB:  75 22                        jne      0x1eff
001EDD:  A1 88 67                     mov      ax, word ptr [0x6788]
001EE0:  9A 00 00 71 09               lcall    0x971, 0
001EE5:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
001EEA:  74 13                        je       0x1eff
001EEC:  1E                           push     ds
001EED:  C5 36 21 52                  lds      si, ptr [0x5221]
001EF1:  9A 3E 0F 99 02               lcall    0x299, 0xf3e
001EF6:  1F                           pop      ds
001EF7:  E8 B5 F8                     call     0x17af
001EFA:  E8 8E F8                     call     0x178b
001EFD:  EB D3                        jmp      0x1ed2
001EFF:  C6 06 13 0B 00               mov      byte ptr [0xb13], 0
001F04:  C6 06 B2 1F 00               mov      byte ptr [0x1fb2], 0
001F09:  C7 06 88 67 FF FF            mov      word ptr [0x6788], 0xffff
001F0F:  C3                           ret     
