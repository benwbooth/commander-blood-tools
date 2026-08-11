; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0087bd
; seg_off: 071e:0fdd
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_choice_handler_2
; label_comment: CONSOLE ROW 2 = THE CONTACT MENU, built from live state, never a fixed list: `mov si,0x6D3E / mov di,0x2B13`, then per word LODSW -- zero means EMPTY SLOT and is SKIPPED (`or ax,ax / je` loops back), 0xFFFF terminates and is stored, otherwise `add ax,4` + STOSW. Record+4 is the object's INLINE NAME (see vm.rs object_inline_name), so the menu is the names of whoever is aboard. Ported as VmMachine::ship_contact_menu_words with a test pinning skip/terminate/name || MERGED 2026-07-25 (audit-fixes #130), also recorded as: special-slot target handler: builds list from DS:0x6D3E, waits interpolation, sets deferred related and input gate || navigation-choice handler 2 (dispatch-table entry 2). PORTED: ship3d.rs run_ship_3d_nav_choice_handler_2
; incoming: nav_choice_subdispatch:choice_2
; byte_count: 139
; boundary: cfg_blocks_14_terminals_2
; terminal: jmp 0x87cb:1, ret:1
; direct_callees: 0x008428
; indirect_calls: 1
; routine_bytes_sha256: 89ce507699583e0e3478943d422fe598e12703aa2d391fb41da5bb93554da9b5

0087BD:  06                           push     es
0087BE:  F6 06 65 25 01               test     byte ptr [0x2565], 1
0087C3:  74 36                        je       0x87fb
0087C5:  BE 3E 6D                     mov      si, 0x6d3e
0087C8:  BF 13 2B                     mov      di, 0x2b13
0087CB:  AD                           lodsw    ax, word ptr [si]
0087CC:  0B C0                        or       ax, ax
0087CE:  74 FB                        je       0x87cb
0087D0:  83 F8 FF                     cmp      ax, -1
0087D3:  74 06                        je       0x87db
0087D5:  83 C0 04                     add      ax, 4
0087D8:  AB                           stosw    word ptr es:[di], ax
0087D9:  EB F0                        jmp      0x87cb
0087DB:  AB                           stosw    word ptr es:[di], ax
0087DC:  66 8E 06 26 67               mov      es, word ptr [0x6726]
0087E1:  BE 13 2B                     mov      si, 0x2b13
0087E4:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
0087E9:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
0087EE:  0E                           push     cs
0087EF:  E8 36 FC                     call     0x8428
0087F2:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
0087F7:  FE 06 65 25                  inc      byte ptr [0x2565]
0087FB:  F6 06 65 25 02               test     byte ptr [0x2565], 2
008800:  74 12                        je       0x8814
008802:  BE AB 2A                     mov      si, 0x2aab
008805:  BF 3D 25                     mov      di, 0x253d
008808:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
00880D:  73 37                        jae      0x8846
00880F:  C6 06 65 25 00               mov      byte ptr [0x2565], 0
008814:  66 8E 06 26 67               mov      es, word ptr [0x6726]
008819:  BE 13 2B                     mov      si, 0x2b13
00881C:  0E                           push     cs
00881D:  E8 08 FC                     call     0x8428
008820:  83 F8 FF                     cmp      ax, -1
008823:  74 21                        je       0x8846
008825:  03 C0                        add      ax, ax
008827:  03 F0                        add      si, ax
008829:  8B 04                        mov      ax, word ptr [si]
00882B:  83 F8 FF                     cmp      ax, -1
00882E:  74 0B                        je       0x883b
008830:  83 E8 04                     sub      ax, 4
008833:  A3 6A 67                     mov      word ptr [0x676a], ax
008836:  C6 06 51 27 01               mov      byte ptr [0x2751], 1
00883B:  C7 06 19 2A 00 00            mov      word ptr [0x2a19], 0
008841:  80 26 93 27 FB               and      byte ptr [0x2793], 0xfb
008846:  07                           pop      es
008847:  C3                           ret     
