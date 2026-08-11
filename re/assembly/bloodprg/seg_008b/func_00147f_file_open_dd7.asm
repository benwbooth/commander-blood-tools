; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00147f
; seg_off: 008b:05cf
; group: seg_008b
; provenance: recursive_graph
; label: file_open_dd7
; label_comment: open a specific file: cx=4; dx=0xdd7 (filename in the string segment); opens the named resource. A dedicated file-open (dd7 = a specific config/data file)
; byte_count: 28
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: df3e0a226e075adb120a93e1abd3cd0a5d166fd1e32dc7ccf91cfc2fa2c7c06d

00147F:  66 33 D2                     xor      edx, edx
001482:  B9 04 00                     mov      cx, 4
001485:  BA D7 0D                     mov      dx, 0xdd7
001488:  51                           push     cx
001489:  67 80 3A 78                  cmp      byte ptr [edx], 0x78
00148D:  74 05                        je       0x1494
00148F:  B8 00 41                     mov      ax, 0x4100
001492:  CD 21                        int      0x21
001494:  83 C2 10                     add      dx, 0x10
001497:  59                           pop      cx
001498:  E2 EE                        loop     0x1488
00149A:  C3                           ret     
