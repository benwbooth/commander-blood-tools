; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a15f
; seg_off: 0971:044f
; group: seg_0971
; provenance: recursive_graph
; label: resource_load_sequence
; label_comment: resource load sequence: call resource_switch 0x9f8e (store id + close old + reinit list); call 0xa642 (banked_list_load); carry-flag error handling. Loads a new banked resource end to end
; byte_count: 85
; boundary: cfg_blocks_6_terminals_1
; terminal: ret:1
; direct_callees: 0x009f8e, 0x00a2ab, 0x00a41a, 0x00a552, 0x00a642, 0x00a757
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a15f_resource_load_sequence.cpp
; routine_bytes_sha256: 3e1ca8aa98aaf77324b482441d46184dea56f924e68737181bede861f493c665

00A15F:  50                           push     ax
00A160:  53                           push     bx
00A161:  51                           push     cx
00A162:  52                           push     dx
00A163:  06                           push     es
00A164:  57                           push     di
00A165:  1E                           push     ds
00A166:  56                           push     si
00A167:  55                           push     bp
00A168:  E8 23 FE                     call     0x9f8e
00A16B:  72 3D                        jb       0xa1aa
00A16D:  E8 D2 04                     call     0xa642
00A170:  72 38                        jb       0xa1aa
00A172:  C4 36 90 0D                  les      si, ptr [0xd90]
00A176:  26 AD                        lodsw    ax, word ptr es:[si]
00A178:  8B 2E BE 0A                  mov      bp, word ptr [0xabe]
00A17C:  E8 D3 03                     call     0xa552
00A17F:  0E                           push     cs
00A180:  E8 97 02                     call     0xa41a
00A183:  0E                           push     cs
00A184:  E8 D0 05                     call     0xa757
00A187:  FF 06 60 0D                  inc      word ptr [0xd60]
00A18B:  FF 06 1C 13                  inc      word ptr [0x131c]
00A18F:  FF 06 62 0D                  inc      word ptr [0xd62]
00A193:  F6 06 76 0D 40               test     byte ptr [0xd76], 0x40
00A198:  75 0A                        jne      0xa1a4
00A19A:  B9 32 00                     mov      cx, 0x32
00A19D:  51                           push     cx
00A19E:  E8 0A 01                     call     0xa2ab
00A1A1:  59                           pop      cx
00A1A2:  E2 F9                        loop     0xa19d
00A1A4:  A1 29 0B                     mov      ax, word ptr [0xb29]
00A1A7:  A3 A2 0D                     mov      word ptr [0xda2], ax
00A1AA:  5D                           pop      bp
00A1AB:  5E                           pop      si
00A1AC:  1F                           pop      ds
00A1AD:  5F                           pop      di
00A1AE:  07                           pop      es
00A1AF:  5A                           pop      dx
00A1B0:  59                           pop      cx
00A1B1:  5B                           pop      bx
00A1B2:  58                           pop      ax
00A1B3:  C3                           ret     
