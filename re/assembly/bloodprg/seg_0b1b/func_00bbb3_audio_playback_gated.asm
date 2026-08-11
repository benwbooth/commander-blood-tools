; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bbb3
; seg_off: 0b1b:0403
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: audio_playback_gated
; label_comment: SND/VOC audio playback+mix family (0xbbb3/0xbc50/0xbdb7): gated on gs:[0xade]&1 (sound enabled) and gs:[0xba3]&1; processes/mixes a sound clip into the audio output. The core software audio-mixing routines
; incoming: call@0x001264->0b1b:0403
; incoming: call@0x001f2f->0b1b:0403
; incoming: call@0x005c1d->0b1b:0403
; incoming: call@0x007b47->0b1b:0403
; incoming: call@0x00891c->0b1b:0403
; incoming: call@0x00b261->0b1b:0403
; byte_count: 157
; boundary: cfg_blocks_8_terminals_1
; terminal: retf:1
; direct_callees: 0x00bb9d, 0x00bd09
; indirect_calls: 1
; routine_bytes_sha256: e2568920091990dad94d80153b8d6981b474cca099647f0ff716b90198dae376

00BBB3:  06                           push     es
00BBB4:  57                           push     di
00BBB5:  1E                           push     ds
00BBB6:  56                           push     si
00BBB7:  50                           push     ax
00BBB8:  53                           push     bx
00BBB9:  65 F6 06 DE 0A 01            test     byte ptr gs:[0xade], 1
00BBBF:  0F 84 86 00                  je       0xbc49
00BBC3:  65 F6 06 A3 0B 01            test     byte ptr gs:[0xba3], 1
00BBC9:  74 7E                        je       0xbc49
00BBCB:  65 F6 06 A0 0B 01            test     byte ptr gs:[0xba0], 1
00BBD1:  75 08                        jne      0xbbdb
00BBD3:  65 F6 06 A0 0B 02            test     byte ptr gs:[0xba0], 2
00BBD9:  74 6E                        je       0xbc49
00BBDB:  8C E8                        mov      ax, gs
00BBDD:  8E D8                        mov      ds, ax
00BBDF:  C6 06 A2 0B 00               mov      byte ptr [0xba2], 0
00BBE4:  BE 89 0B                     mov      si, 0xb89
00BBE7:  C4 3E B7 0B                  les      di, ptr [0xbb7]
00BBEB:  89 3C                        mov      word ptr [si], di
00BBED:  8C 44 02                     mov      word ptr [si + 2], es
00BBF0:  C7 44 04 00 40               mov      word ptr [si + 4], 0x4000
00BBF5:  33 C0                        xor      ax, ax
00BBF7:  E8 0F 01                     call     0xbd09
00BBFA:  40                           inc      ax
00BBFB:  A3 A5 0B                     mov      word ptr [0xba5], ax
00BBFE:  26 80 7D 04 D3               cmp      byte ptr es:[di + 4], 0xd3
00BC03:  75 05                        jne      0xbc0a
00BC05:  C6 06 A2 0B 01               mov      byte ptr [0xba2], 1
00BC0A:  26 8B 05                     mov      ax, word ptr es:[di]
00BC0D:  A3 99 0B                     mov      word ptr [0xb99], ax
00BC10:  26 8B 45 02                  mov      ax, word ptr es:[di + 2]
00BC14:  A3 9B 0B                     mov      word ptr [0xb9b], ax
00BC17:  26 8B 45 04                  mov      ax, word ptr es:[di + 4]
00BC1B:  A3 9D 0B                     mov      word ptr [0xb9d], ax
00BC1E:  81 C7 08 40                  add      di, 0x4008
00BC22:  BE 91 0B                     mov      si, 0xb91
00BC25:  89 3C                        mov      word ptr [si], di
00BC27:  8C 44 02                     mov      word ptr [si + 2], es
00BC2A:  C7 44 04 00 40               mov      word ptr [si + 4], 0x4000
00BC2F:  C6 44 06 00                  mov      byte ptr [si + 6], 0
00BC33:  0E                           push     cs
00BC34:  E8 66 FF                     call     0xbb9d
00BC37:  C6 06 A0 0B 02               mov      byte ptr [0xba0], 2
00BC3C:  BE 89 0B                     mov      si, 0xb89
00BC3F:  33 C0                        xor      ax, ax
00BC41:  C6 44 06 01                  mov      byte ptr [si + 6], 1
00BC45:  FF 1E DB 0C                  lcall    [0xcdb]
00BC49:  5B                           pop      bx
00BC4A:  58                           pop      ax
00BC4B:  5E                           pop      si
00BC4C:  1F                           pop      ds
00BC4D:  5F                           pop      di
00BC4E:  07                           pop      es
00BC4F:  CB                           retf    
