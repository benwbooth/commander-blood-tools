; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0077a9
; seg_off: 04da:2409
; group: seg_04da
; provenance: static_dispatch_table_target
; label: music_voc_name_patcher
; label_comment: Patches the MUSIC filename into the `mu\\xxxxxxxx.voc` template at DS:0x0D2C, writing the 8-char name field at DS:0x0D30. Copies from ds:si uppercasing (cmp al,0x61 / and al,0xDF), stopping at space, control or a high-bit byte; sets gs:0xBA1 if any character CHANGED and gs:0xBA0 if the name was identical, so the caller can skip a reload. RESOLVED 2026-07-24: this is MUSIC, not dialogue voice -- all 44 files in BLOOD.DAT's mu/ are track names (angst, blintr, credits, croolrap, tablo2, generik...) and NONE is a character name. Dialogue voices come from the son.snd bank instead (loader 0xC005, handle DS:0x0C47)
; incoming: byte_parser_dispatch_74e5:byte_0x12
; byte_count: 52
; boundary: cfg_blocks_11_terminals_2
; terminal: jmp 0x77ac:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0077a9_music_voc_name_patcher.cpp
; routine_bytes_sha256: 2177b4dd9c7763c956100260a38cb70600c06256437e351830023f135f4cbf4e

0077A9:  BF 30 0D                     mov      di, 0xd30
0077AC:  AC                           lodsb    al, byte ptr [si]
0077AD:  0A C0                        or       al, al
0077AF:  78 18                        js       0x77c9
0077B1:  3C 20                        cmp      al, 0x20
0077B3:  76 14                        jbe      0x77c9
0077B5:  3C 61                        cmp      al, 0x61
0077B7:  72 02                        jb       0x77bb
0077B9:  24 DF                        and      al, 0xdf
0077BB:  26 3A 05                     cmp      al, byte ptr es:[di]
0077BE:  74 06                        je       0x77c6
0077C0:  65 C6 06 A1 0B 01            mov      byte ptr gs:[0xba1], 1
0077C6:  AA                           stosb    byte ptr es:[di], al
0077C7:  EB E3                        jmp      0x77ac
0077C9:  65 F6 06 A1 0B 01            test     byte ptr gs:[0xba1], 1
0077CF:  75 06                        jne      0x77d7
0077D1:  65 80 0E A0 0B 01            or       byte ptr gs:[0xba0], 1
0077D7:  4E                           dec      si
0077D8:  26 C6 05 00                  mov      byte ptr es:[di], 0
0077DC:  C3                           ret     
