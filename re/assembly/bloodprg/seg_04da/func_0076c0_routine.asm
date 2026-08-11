; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0076c0
; seg_off: 04da:2320
; group: seg_04da
; provenance: static_dispatch_table_target
; incoming: byte_parser_dispatch_74e5:byte_0x09
; byte_count: 21
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x76c3:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 3d2a39d63f78374292613ff1c317b5bc875ab4109bd5f683456ab48e813471b2

0076C0:  BF 60 24                     mov      di, 0x2460
0076C3:  AC                           lodsb    al, byte ptr [si]
0076C4:  0A C0                        or       al, al
0076C6:  78 07                        js       0x76cf
0076C8:  3C 20                        cmp      al, 0x20
0076CA:  72 03                        jb       0x76cf
0076CC:  AA                           stosb    byte ptr es:[di], al
0076CD:  EB F4                        jmp      0x76c3
0076CF:  4E                           dec      si
0076D0:  26 C6 05 00                  mov      byte ptr es:[di], 0
0076D4:  C3                           ret     
