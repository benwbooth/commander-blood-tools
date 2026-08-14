; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001855
; seg_off: 008b:09a5
; group: seg_008b
; provenance: recursive_graph
; label: scene_transition_step
; label_comment: priority-ordered scene-transition state machine. Bit 0 arms DS:0x5249; phase 1 initializes record and presentation state; bits 1/2/3/4 select image/palette load, deferred-record arming, bridge/alien coordination, and finish, followed by full presentation cleanup. Direct vectors: re/tools/oracle_vectors/func_1855_natural.json
; byte_count: 574
; boundary: cfg_blocks_31_terminals_10
; terminal: jmp 0x1a8e:9, ret:1
; direct_callees: none
; indirect_calls: 11
; routine_bytes_sha256: 29035d247ed5d6e49f85d71625ad49187155d5ddb69a56e4d50e2fd45e7d062b

001855:  50                           push     ax
001856:  06                           push     es
001857:  57                           push     di
001858:  56                           push     si
001859:  A0 51 27                     mov      al, byte ptr [0x2751]
00185C:  A8 01                        test     al, 1
00185E:  0F 84 2C 02                  je       0x1a8e
001862:  C7 06 49 52 01 00            mov      word ptr [0x5249], 1
001868:  A8 FE                        test     al, 0xfe
00186A:  75 39                        jne      0x18a5
00186C:  B8 04 00                     mov      ax, 4
00186F:  9A 41 12 99 02               lcall    0x299, 0x1241
001874:  B8 1F 00                     mov      ax, 0x1f
001877:  9A 41 12 99 02               lcall    0x299, 0x1241
00187C:  C7 06 93 27 00 00            mov      word ptr [0x2793], 0
001882:  80 0E 51 27 02               or       byte ptr [0x2751], 2
001887:  C7 06 88 67 29 00            mov      word ptr [0x6788], 0x29
00188D:  8B 3E 6A 67                  mov      di, word ptr [0x676a]
001891:  89 3E 4D 27                  mov      word ptr [0x274d], di
001895:  66 8E 06 26 67               mov      es, word ptr [0x6726]
00189A:  83 C7 04                     add      di, 4
00189D:  9A 69 20 DA 04               lcall    0x4da, 0x2069
0018A2:  E9 E9 01                     jmp      0x1a8e
0018A5:  9A 00 00 71 09               lcall    0x971, 0
0018AA:  A8 02                        test     al, 2
0018AC:  0F 84 B8 00                  je       0x1968
0018B0:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
0018B5:  0F 85 D5 01                  jne      0x1a8e
0018B9:  C6 06 51 27 05               mov      byte ptr [0x2751], 5
0018BE:  C7 06 A7 1F 23 00            mov      word ptr [0x1fa7], 0x23
0018C4:  C6 06 4F 27 01               mov      byte ptr [0x274f], 1
0018C9:  BE F3 00                     mov      si, 0xf3
0018CC:  C4 3E 29 52                  les      di, ptr [0x5229]
0018D0:  C6 06 53 5B 01               mov      byte ptr [0x5b53], 1
0018D5:  C6 06 57 5B 00               mov      byte ptr [0x5b57], 0
0018DA:  9A 1D 09 CE 01               lcall    0x1ce, 0x91d
0018DF:  1E                           push     ds
0018E0:  C5 36 29 52                  lds      si, ptr [0x5229]
0018E4:  9A B6 0E 99 02               lcall    0x299, 0xeb6
0018E9:  1F                           pop      ds
0018EA:  8B 3E 4D 27                  mov      di, word ptr [0x274d]
0018EE:  66 8E 06 26 67               mov      es, word ptr [0x6726]
0018F3:  26 83 3D 02                  cmp      word ptr es:[di], 2
0018F7:  74 33                        je       0x192c
0018F9:  C7 06 32 0A FF FF            mov      word ptr [0xa32], 0xffff
0018FF:  33 C0                        xor      ax, ax
001901:  C7 06 39 52 23 00            mov      word ptr [0x5239], 0x23
001907:  C7 06 3B 52 A5 00            mov      word ptr [0x523b], 0xa5
00190D:  9A 2F 0E 99 02               lcall    0x299, 0xe2f
001912:  C7 06 39 52 00 00            mov      word ptr [0x5239], 0
001918:  C7 06 3B 52 C8 00            mov      word ptr [0x523b], 0xc8
00191E:  C6 06 51 27 09               mov      byte ptr [0x2751], 9
001923:  C7 06 88 67 2B 00            mov      word ptr [0x6788], 0x2b
001929:  E9 62 01                     jmp      0x1a8e
00192C:  8C E8                        mov      ax, gs
00192E:  8E C0                        mov      es, ax
001930:  BE D1 53                     mov      si, 0x53d1
001933:  BF D1 56                     mov      di, 0x56d1
001936:  B9 30 00                     mov      cx, 0x30
001939:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00193C:  BE D1 53                     mov      si, 0x53d1
00193F:  BF D1 59                     mov      di, 0x59d1
001942:  B9 C0 00                     mov      cx, 0xc0
001945:  AC                           lodsb    al, byte ptr [si]
001946:  2C 28                        sub      al, 0x28
001948:  79 02                        jns      0x194c
00194A:  32 C0                        xor      al, al
00194C:  AA                           stosb    byte ptr es:[di], al
00194D:  E2 F6                        loop     0x1945
00194F:  C6 06 51 5B 80               mov      byte ptr [0x5b51], 0x80
001954:  C6 06 52 5B BF               mov      byte ptr [0x5b52], 0xbf
001959:  C7 06 4D 52 05 00            mov      word ptr [0x524d], 5
00195F:  C7 06 88 67 27 00            mov      word ptr [0x6788], 0x27
001965:  E9 26 01                     jmp      0x1a8e
001968:  A8 04                        test     al, 4
00196A:  74 1D                        je       0x1989
00196C:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
001971:  0F 85 19 01                  jne      0x1a8e
001975:  C7 06 68 67 C4 00            mov      word ptr [0x6768], 0xc4
00197B:  C6 06 51 27 89               mov      byte ptr [0x2751], 0x89
001980:  C7 06 32 0A 00 00            mov      word ptr [0xa32], 0
001986:  E9 05 01                     jmp      0x1a8e
001989:  A8 08                        test     al, 8
00198B:  0F 84 9D 00                  je       0x1a2c
00198F:  9A 76 1E 1E 07               lcall    0x71e, 0x1e76
001994:  8B 3E 4D 27                  mov      di, word ptr [0x274d]
001998:  66 8E 06 26 67               mov      es, word ptr [0x6726]
00199D:  26 83 3D 02                  cmp      word ptr es:[di], 2
0019A1:  74 0C                        je       0x19af
0019A3:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
0019A8:  0F 84 8B 00                  je       0x1a37
0019AC:  E9 DF 00                     jmp      0x1a8e
0019AF:  F6 06 51 27 80               test     byte ptr [0x2751], 0x80
0019B4:  0F 85 D6 00                  jne      0x1a8e
0019B8:  83 3E 88 67 07               cmp      word ptr [0x6788], 7
0019BD:  74 66                        je       0x1a25
0019BF:  F6 06 51 27 40               test     byte ptr [0x2751], 0x40
0019C4:  74 19                        je       0x19df
0019C6:  80 26 51 27 BF               and      byte ptr [0x2751], 0xbf
0019CB:  BE F3 00                     mov      si, 0xf3
0019CE:  C4 3E 29 52                  les      di, ptr [0x5229]
0019D2:  C6 06 53 5B 00               mov      byte ptr [0x5b53], 0
0019D7:  9A 1D 09 CE 01               lcall    0x1ce, 0x91d
0019DC:  E9 AF 00                     jmp      0x1a8e
0019DF:  9A F1 05 9A 0A               lcall    0xa9a, 0x5f1
0019E4:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
0019E9:  0F 85 A1 00                  jne      0x1a8e
0019ED:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
0019F2:  0F 85 98 00                  jne      0x1a8e
0019F6:  C6 06 51 27 11               mov      byte ptr [0x2751], 0x11
0019FB:  C7 06 88 67 28 00            mov      word ptr [0x6788], 0x28
001A01:  8C E8                        mov      ax, gs
001A03:  8E C0                        mov      es, ax
001A05:  BE D1 56                     mov      si, 0x56d1
001A08:  BF D1 59                     mov      di, 0x59d1
001A0B:  B9 30 00                     mov      cx, 0x30
001A0E:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
001A11:  BE D1 53                     mov      si, 0x53d1
001A14:  BF D1 56                     mov      di, 0x56d1
001A17:  B9 30 00                     mov      cx, 0x30
001A1A:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
001A1D:  C7 06 4F 52 00 00            mov      word ptr [0x524f], 0
001A23:  EB 69                        jmp      0x1a8e
001A25:  80 0E 51 27 40               or       byte ptr [0x2751], 0x40
001A2A:  EB 62                        jmp      0x1a8e
001A2C:  A8 10                        test     al, 0x10
001A2E:  74 1F                        je       0x1a4f
001A30:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
001A35:  75 57                        jne      0x1a8e
001A37:  C7 06 A7 1F 00 00            mov      word ptr [0x1fa7], 0
001A3D:  C6 06 51 27 21               mov      byte ptr [0x2751], 0x21
001A42:  C7 06 88 67 2A 00            mov      word ptr [0x6788], 0x2a
001A48:  C6 06 4F 27 00               mov      byte ptr [0x274f], 0
001A4D:  EB 3F                        jmp      0x1a8e
001A4F:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
001A54:  75 38                        jne      0x1a8e
001A56:  33 C0                        xor      ax, ax
001A58:  A3 32 0A                     mov      word ptr [0xa32], ax
001A5B:  A2 51 27                     mov      byte ptr [0x2751], al
001A5E:  C7 06 93 27 01 00            mov      word ptr [0x2793], 1
001A64:  C7 06 AB 1F FF FF            mov      word ptr [0x1fab], 0xffff
001A6A:  C7 06 88 67 FF FF            mov      word ptr [0x6788], 0xffff
001A70:  A2 B2 1F                     mov      byte ptr [0x1fb2], al
001A73:  A2 64 5E                     mov      byte ptr [0x5e64], al
001A76:  A2 B0 67                     mov      byte ptr [0x67b0], al
001A79:  A2 BC 67                     mov      byte ptr [0x67bc], al
001A7C:  80 26 AA 67 FC               and      byte ptr [0x67aa], 0xfc
001A81:  A2 BA 67                     mov      byte ptr [0x67ba], al
001A84:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
001A89:  9A B6 14 1E 07               lcall    0x71e, 0x14b6
001A8E:  5E                           pop      si
001A8F:  5F                           pop      di
001A90:  07                           pop      es
001A91:  58                           pop      ax
001A92:  C3                           ret     
