; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00872c
; seg_off: 071e:0f4c
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_choice_handler_1
; label_comment: target-list choice handler: adjusts target records, waits interpolation, defers C3 target, reloads radio.snd
; incoming: nav_choice_subdispatch:choice_1
; byte_count: 145
; boundary: cfg_blocks_13_terminals_2
; terminal: jmp 0x8748:1, ret:1
; direct_callees: 0x008428
; indirect_calls: 3
; cxx_source: re/borland/bloodprg/seg_071e/func_00872c_nav_choice_handler_1.cpp
; routine_bytes_sha256: 472d17fbac8b5275f1258a655fb63eb38ed3e9f33ca9677c763682b74653ecad

00872C:  06                           push     es
00872D:  66 8E 06 26 67               mov      es, word ptr [0x6726]
008732:  BE 13 2B                     mov      si, 0x2b13
008735:  F6 06 65 25 01               test     byte ptr [0x2565], 1
00873A:  74 2E                        je       0x876a
00873C:  9A 2F 1E DA 04               lcall    0x4da, 0x1e2f
008741:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
008746:  8B FE                        mov      di, si
008748:  AD                           lodsw    ax, word ptr [si]
008749:  83 F8 FF                     cmp      ax, -1
00874C:  74 08                        je       0x8756
00874E:  83 C0 04                     add      ax, 4
008751:  89 44 FE                     mov      word ptr [si - 2], ax
008754:  EB F2                        jmp      0x8748
008756:  8B F7                        mov      si, di
008758:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
00875D:  0E                           push     cs
00875E:  E8 C7 FC                     call     0x8428
008761:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
008766:  FE 06 65 25                  inc      byte ptr [0x2565]
00876A:  F6 06 65 25 02               test     byte ptr [0x2565], 2
00876F:  74 14                        je       0x8785
008771:  56                           push     si
008772:  BE AB 2A                     mov      si, 0x2aab
008775:  BF 3D 25                     mov      di, 0x253d
008778:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
00877D:  5E                           pop      si
00877E:  73 3B                        jae      0x87bb
008780:  C6 06 65 25 00               mov      byte ptr [0x2565], 0
008785:  0E                           push     cs
008786:  E8 9F FC                     call     0x8428
008789:  83 F8 FF                     cmp      ax, -1
00878C:  74 2D                        je       0x87bb
00878E:  03 C0                        add      ax, ax
008790:  03 F0                        add      si, ax
008792:  8B 04                        mov      ax, word ptr [si]
008794:  83 F8 FF                     cmp      ax, -1
008797:  74 17                        je       0x87b0
008799:  83 E8 04                     sub      ax, 4
00879C:  A3 6A 67                     mov      word ptr [0x676a], ax
00879F:  C7 06 68 67 C3 00            mov      word ptr [0x6768], 0xc3
0087A5:  B8 01 00                     mov      ax, 1
0087A8:  BE 16 0D                     mov      si, 0xd16
0087AB:  9A 55 08 1B 0B               lcall    0xb1b, 0x855
0087B0:  C7 06 19 2A 00 00            mov      word ptr [0x2a19], 0
0087B6:  80 26 93 27 FB               and      byte ptr [0x2793], 0xfb
0087BB:  07                           pop      es
0087BC:  C3                           ret     
