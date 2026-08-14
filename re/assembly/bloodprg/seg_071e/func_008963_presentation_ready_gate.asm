; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008963
; seg_off: 071e:1183
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: presentation_ready_gate
; label_comment: four-stage presentation word-choice coordinator: gates DS:0x67F8, lays it out using the DIC segment, animates open, captures a signed selection, animates closed, then publishes DS:0x6762 and clears presentation state
; natural_c: re/source/bloodprg/candidates/seg_071e/func_008963_presentation_ready_gate.c
; incoming: call@0x0012a8->071e:1183
; byte_count: 235
; boundary: cfg_blocks_15_terminals_2
; terminal: jmp 0x8a49:1, retf:1
; direct_callees: 0x008428
; indirect_calls: 2
; routine_bytes_sha256: d32b63e352178f5e4f668594a96cce5d350b1ae048f3aeb10c374d327a41ccee

008963:  50                           push     ax
008964:  06                           push     es
008965:  56                           push     si
008966:  57                           push     di
008967:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
00896C:  0F 84 D9 00                  je       0x8a49
008970:  F6 06 D7 27 01               test     byte ptr [0x27d7], 1
008975:  0F 84 D0 00                  je       0x8a49
008979:  F6 06 AA 67 02               test     byte ptr [0x67aa], 2
00897E:  0F 85 C7 00                  jne      0x8a49
008982:  BE F8 67                     mov      si, 0x67f8
008985:  83 3C 00                     cmp      word ptr [si], 0
008988:  0F 84 BD 00                  je       0x8a49
00898C:  66 8E 06 2A 67               mov      es, word ptr [0x672a]
008991:  F6 06 BA 67 07               test     byte ptr [0x67ba], 7
008996:  75 3D                        jne      0x89d5
008998:  80 0E 93 27 04               or       byte ptr [0x2793], 4
00899D:  FE 06 BA 67                  inc      byte ptr [0x67ba]
0089A1:  C6 06 DC 0A 00               mov      byte ptr [0xadc], 0
0089A6:  C7 06 C6 0A E1 00            mov      word ptr [0xac6], 0xe1
0089AC:  C6 06 DD 0A 00               mov      byte ptr [0xadd], 0
0089B1:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
0089B6:  C6 06 DA 0A 04               mov      byte ptr [0xada], 4
0089BB:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
0089C0:  0E                           push     cs
0089C1:  E8 64 FA                     call     0x8428
0089C4:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
0089C9:  A1 AB 2A                     mov      ax, word ptr [0x2aab]
0089CC:  A3 4D 25                     mov      word ptr [0x254d], ax
0089CF:  A1 AF 2A                     mov      ax, word ptr [0x2aaf]
0089D2:  A3 51 25                     mov      word ptr [0x2551], ax
0089D5:  F6 06 BA 67 02               test     byte ptr [0x67ba], 2
0089DA:  75 13                        jne      0x89ef
0089DC:  56                           push     si
0089DD:  BE AB 2A                     mov      si, 0x2aab
0089E0:  BF 4D 25                     mov      di, 0x254d
0089E3:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
0089E8:  5E                           pop      si
0089E9:  73 5E                        jae      0x8a49
0089EB:  FE 06 BA 67                  inc      byte ptr [0x67ba]
0089EF:  F6 06 BA 67 01               test     byte ptr [0x67ba], 1
0089F4:  75 1C                        jne      0x8a12
0089F6:  0E                           push     cs
0089F7:  E8 2E FA                     call     0x8428
0089FA:  0B C0                        or       ax, ax
0089FC:  78 4B                        js       0x8a49
0089FE:  03 C0                        add      ax, ax
008A00:  03 F0                        add      si, ax
008A02:  8B 04                        mov      ax, word ptr [si]
008A04:  A3 96 67                     mov      word ptr [0x6796], ax
008A07:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
008A0C:  FE 06 BA 67                  inc      byte ptr [0x67ba]
008A10:  EB 37                        jmp      0x8a49
008A12:  BE 4D 25                     mov      si, 0x254d
008A15:  BF AB 2A                     mov      di, 0x2aab
008A18:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
008A1D:  73 2A                        jae      0x8a49
008A1F:  A1 96 67                     mov      ax, word ptr [0x6796]
008A22:  A3 62 67                     mov      word ptr [0x6762], ax
008A25:  C6 06 D7 27 00               mov      byte ptr [0x27d7], 0
008A2A:  C6 06 B0 67 00               mov      byte ptr [0x67b0], 0
008A2F:  C6 06 64 5E 00               mov      byte ptr [0x5e64], 0
008A34:  C6 06 BB 67 00               mov      byte ptr [0x67bb], 0
008A39:  C6 06 BA 67 00               mov      byte ptr [0x67ba], 0
008A3E:  C7 06 F8 67 00 00            mov      word ptr [0x67f8], 0
008A44:  80 26 AA 67 FE               and      byte ptr [0x67aa], 0xfe
008A49:  5F                           pop      di
008A4A:  5E                           pop      si
008A4B:  07                           pop      es
008A4C:  58                           pop      ax
008A4D:  CB                           retf    
