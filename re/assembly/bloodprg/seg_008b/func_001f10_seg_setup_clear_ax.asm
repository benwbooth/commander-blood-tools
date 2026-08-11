; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001f10
; seg_off: 008b:1060
; group: seg_008b
; provenance: recursive_graph
; label: seg_setup_clear_ax
; label_comment: segment setup: ds=es=gs; xor ax,ax. Rebases DS/ES to the work arena and zeroes ax before a data operation
; byte_count: 104
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x1f45:1, ret:1
; direct_callees: 0x00178b, 0x0017af, 0x00210e
; indirect_calls: 8
; cxx_source: re/borland/bloodprg/seg_008b/func_001f10_seg_setup_clear_ax.cpp
; routine_bytes_sha256: 884c493750fa6ba16c66642e3acf546ee0e0c831a153c1a246c32462360c1459

001F10:  8C E8                        mov      ax, gs
001F12:  8E D8                        mov      ds, ax
001F14:  8E C0                        mov      es, ax
001F16:  33 C0                        xor      ax, ax
001F18:  A2 13 0B                     mov      byte ptr [0xb13], al
001F1B:  A2 2E 25                     mov      byte ptr [0x252e], al
001F1E:  A3 A7 1F                     mov      word ptr [0x1fa7], ax
001F21:  C7 06 88 67 01 00            mov      word ptr [0x6788], 1
001F27:  BE 4B 0D                     mov      si, 0xd4b
001F2A:  9A 07 06 1B 0B               lcall    0xb1b, 0x607
001F2F:  9A 03 04 1B 0B               lcall    0xb1b, 0x403
001F34:  9A 16 00 99 02               lcall    0x299, 0x16
001F39:  9A EB 0D 99 02               lcall    0x299, 0xdeb
001F3E:  33 C0                        xor      ax, ax
001F40:  9A 2F 0E 99 02               lcall    0x299, 0xe2f
001F45:  0E                           push     cs
001F46:  E8 C5 01                     call     0x210e
001F49:  F6 06 13 0B 01               test     byte ptr [0xb13], 1
001F4E:  75 27                        jne      0x1f77
001F50:  A1 88 67                     mov      ax, word ptr [0x6788]
001F53:  9A 00 00 71 09               lcall    0x971, 0
001F58:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
001F5D:  74 18                        je       0x1f77
001F5F:  9A A0 04 1B 0B               lcall    0xb1b, 0x4a0
001F64:  1E                           push     ds
001F65:  C5 36 21 52                  lds      si, ptr [0x5221]
001F69:  9A 3E 0F 99 02               lcall    0x299, 0xf3e
001F6E:  1F                           pop      ds
001F6F:  E8 3D F8                     call     0x17af
001F72:  E8 16 F8                     call     0x178b
001F75:  EB CE                        jmp      0x1f45
001F77:  C3                           ret     
