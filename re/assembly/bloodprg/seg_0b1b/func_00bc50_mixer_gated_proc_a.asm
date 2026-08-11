; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bc50
; seg_off: 0b1b:04a0
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: mixer_gated_proc_a
; label_comment: mixer-gated audio routine: test gs:[0xade],1 (software-mixer enable gate) AND test gs:[0xba3],1 (second audio-active flag); both must be set or it bails to 0xbd01. Runs the per-frame mix step only when the mixer and channel are live
; incoming: call@0x001271->0b1b:04a0
; incoming: call@0x001f5f->0b1b:04a0
; byte_count: 185
; boundary: cfg_blocks_19_terminals_3
; terminal: jmp 0xbc79:2, retf:1
; direct_callees: 0x00bd09
; indirect_calls: 3
; cxx_source: re/borland/bloodprg/seg_0b1b/func_00bc50_mixer_gated_proc_a.cpp
; routine_bytes_sha256: 99c96376cc75ea56f0a0d77f7540549d10bfc80eac3e1be93a131a71a5b9a765

00BC50:  06                           push     es
00BC51:  57                           push     di
00BC52:  56                           push     si
00BC53:  50                           push     ax
00BC54:  53                           push     bx
00BC55:  1E                           push     ds
00BC56:  52                           push     dx
00BC57:  65 F6 06 DE 0A 01            test     byte ptr gs:[0xade], 1
00BC5D:  0F 84 A0 00                  je       0xbd01
00BC61:  65 F6 06 A3 0B 01            test     byte ptr gs:[0xba3], 1
00BC67:  0F 84 96 00                  je       0xbd01
00BC6B:  65 F6 06 A0 0B 02            test     byte ptr gs:[0xba0], 2
00BC71:  0F 84 8C 00                  je       0xbd01
00BC75:  8C E8                        mov      ax, gs
00BC77:  8E D8                        mov      ds, ax
00BC79:  FF 1E F3 0C                  lcall    [0xcf3]
00BC7D:  8B D8                        mov      bx, ax
00BC7F:  BE 89 0B                     mov      si, 0xb89
00BC82:  F6 44 06 02                  test     byte ptr [si + 6], 2
00BC86:  74 12                        je       0xbc9a
00BC88:  83 C6 08                     add      si, 8
00BC8B:  F6 44 06 02                  test     byte ptr [si + 6], 2
00BC8F:  74 09                        je       0xbc9a
00BC91:  0B DB                        or       bx, bx
00BC93:  74 05                        je       0xbc9a
00BC95:  83 FB FF                     cmp      bx, -1
00BC98:  75 67                        jne      0xbd01
00BC9A:  C4 3C                        les      di, ptr [si]
00BC9C:  A1 A5 0B                     mov      ax, word ptr [0xba5]
00BC9F:  0B C0                        or       ax, ax
00BCA1:  74 0E                        je       0xbcb1
00BCA3:  50                           push     ax
00BCA4:  A1 99 0B                     mov      ax, word ptr [0xb99]
00BCA7:  AB                           stosw    word ptr es:[di], ax
00BCA8:  A1 9B 0B                     mov      ax, word ptr [0xb9b]
00BCAB:  AB                           stosw    word ptr es:[di], ax
00BCAC:  A1 9D 0B                     mov      ax, word ptr [0xb9d]
00BCAF:  AB                           stosw    word ptr es:[di], ax
00BCB0:  58                           pop      ax
00BCB1:  E8 55 00                     call     0xbd09
00BCB4:  C6 44 06 01                  mov      byte ptr [si + 6], 1
00BCB8:  C7 44 04 00 40               mov      word ptr [si + 4], 0x4000
00BCBD:  A1 A5 0B                     mov      ax, word ptr [0xba5]
00BCC0:  40                           inc      ax
00BCC1:  3B 06 A7 0B                  cmp      ax, word ptr [0xba7]
00BCC5:  72 08                        jb       0xbccf
00BCC7:  A1 A9 0B                     mov      ax, word ptr [0xba9]
00BCCA:  89 44 04                     mov      word ptr [si + 4], ax
00BCCD:  33 C0                        xor      ax, ax
00BCCF:  A3 A5 0B                     mov      word ptr [0xba5], ax
00BCD2:  33 C0                        xor      ax, ax
00BCD4:  0B DB                        or       bx, bx
00BCD6:  74 0B                        je       0xbce3
00BCD8:  83 FB FF                     cmp      bx, -1
00BCDB:  74 06                        je       0xbce3
00BCDD:  FF 1E EB 0C                  lcall    [0xceb]
00BCE1:  EB 96                        jmp      0xbc79
00BCE3:  81 FE 89 0B                  cmp      si, 0xb89
00BCE7:  0F 94 06 8F 0B               sete     byte ptr [0xb8f]
00BCEC:  0F 95 06 97 0B               setne    byte ptr [0xb97]
00BCF1:  3B 3E B7 0B                  cmp      di, word ptr [0xbb7]
00BCF5:  74 03                        je       0xbcfa
00BCF7:  83 EF 06                     sub      di, 6
00BCFA:  FF 1E DB 0C                  lcall    [0xcdb]
00BCFE:  E9 78 FF                     jmp      0xbc79
00BD01:  5A                           pop      dx
00BD02:  1F                           pop      ds
00BD03:  5B                           pop      bx
00BD04:  58                           pop      ax
00BD05:  5E                           pop      si
00BD06:  5F                           pop      di
00BD07:  07                           pop      es
00BD08:  CB                           retf    
