; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b2bb
; seg_off: 0a9a:031b
; group: seg_0a9a
; provenance: recursive_graph
; label: ship_3d_target_record_select
; label_comment: selects next ship/navigation target record from DS:0x250B or fallback DS:0x2537
; byte_count: 147
; boundary: cfg_blocks_14_terminals_3
; terminal: jmp 0xb34a:2, ret:1
; direct_callees: none
; indirect_calls: 3
; routine_bytes_sha256: dded91c3049c7a8551314c036c3bbe1531ac79264cbcf315d73575e76cb3a887

00B2BB:  56                           push     si
00B2BC:  06                           push     es
00B2BD:  57                           push     di
00B2BE:  C6 06 2C 25 00               mov      byte ptr [0x252c], 0
00B2C3:  BE 0B 25                     mov      si, 0x250b
00B2C6:  66 8E 06 26 67               mov      es, word ptr [0x6726]
00B2CB:  83 3C FF                     cmp      word ptr [si], -1
00B2CE:  75 0C                        jne      0xb2dc
00B2D0:  8C D8                        mov      ax, ds
00B2D2:  8E C0                        mov      es, ax
00B2D4:  BE 37 25                     mov      si, 0x2537
00B2D7:  C6 06 2C 25 01               mov      byte ptr [0x252c], 1
00B2DC:  F6 06 2B 25 01               test     byte ptr [0x252b], 1
00B2E1:  74 18                        je       0xb2fb
00B2E3:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
00B2E8:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
00B2ED:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
00B2F2:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
00B2F7:  FE 06 2B 25                  inc      byte ptr [0x252b]
00B2FB:  33 C0                        xor      ax, ax
00B2FD:  F6 06 2B 25 02               test     byte ptr [0x252b], 2
00B302:  74 14                        je       0xb318
00B304:  56                           push     si
00B305:  BE AB 2A                     mov      si, 0x2aab
00B308:  BF 45 25                     mov      di, 0x2545
00B30B:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
00B310:  5E                           pop      si
00B311:  73 37                        jae      0xb34a
00B313:  C6 06 2B 25 00               mov      byte ptr [0x252b], 0
00B318:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
00B31D:  83 F8 FF                     cmp      ax, -1
00B320:  75 04                        jne      0xb326
00B322:  33 C0                        xor      ax, ax
00B324:  EB 24                        jmp      0xb34a
00B326:  03 C0                        add      ax, ax
00B328:  03 F0                        add      si, ax
00B32A:  8B 04                        mov      ax, word ptr [si]
00B32C:  83 F8 FF                     cmp      ax, -1
00B32F:  75 0C                        jne      0xb33d
00B331:  C6 06 2F 25 01               mov      byte ptr [0x252f], 1
00B336:  C6 06 31 25 06               mov      byte ptr [0x2531], 6
00B33B:  EB 0D                        jmp      0xb34a
00B33D:  83 E8 04                     sub      ax, 4
00B340:  F6 06 2C 25 01               test     byte ptr [0x252c], 1
00B345:  74 03                        je       0xb34a
00B347:  A1 1B 25                     mov      ax, word ptr [0x251b]
00B34A:  5F                           pop      di
00B34B:  07                           pop      es
00B34C:  5E                           pop      si
00B34D:  C3                           ret     
