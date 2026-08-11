; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008a4e
; seg_off: 071e:126e
; group: seg_071e
; provenance: recursive_graph
; label: camera_fsm_state_gate
; label_comment: camera-approach FSM gate: test [0x27df]&7 (the DS:0x27DF camera-approach phase, ship3d.rs Ship3dCameraApproach); set [0xa32]=1, clear [0x278a]. Gates on the camera state machine phase
; byte_count: 349
; boundary: cfg_blocks_24_terminals_9
; terminal: jmp 0x8b95:5, jmp 0x8ba8:3, ret:1
; direct_callees: 0x008c96, 0x00959d, 0x0098b9, 0x009a10, 0x009b98
; indirect_calls: 5
; cxx_source: re/borland/bloodprg/seg_071e/func_008a4e_camera_fsm_state_gate.cpp
; routine_bytes_sha256: a18703e4df1d18484136dad0121e300f341d951bf379e9ffdc7dc35d13421baf

008A4E:  53                           push     bx
008A4F:  55                           push     bp
008A50:  F6 06 DF 27 07               test     byte ptr [0x27df], 7
008A55:  75 1C                        jne      0x8a73
008A57:  C7 06 32 0A 01 00            mov      word ptr [0xa32], 1
008A5D:  C6 06 8A 27 00               mov      byte ptr [0x278a], 0
008A62:  C6 06 DA 27 01               mov      byte ptr [0x27da], 1
008A67:  E8 33 0B                     call     0x959d
008A6A:  FE 06 DF 27                  inc      byte ptr [0x27df]
008A6E:  83 0E 93 27 04               or       word ptr [0x2793], 4
008A73:  A0 DF 27                     mov      al, byte ptr [0x27df]
008A76:  3C 01                        cmp      al, 1
008A78:  75 39                        jne      0x8ab3
008A7A:  A1 65 2F                     mov      ax, word ptr [0x2f65]
008A7D:  3D 28 23                     cmp      ax, 0x2328
008A80:  7C 2A                        jl       0x8aac
008A82:  83 E8 64                     sub      ax, 0x64
008A85:  A3 65 2F                     mov      word ptr [0x2f65], ax
008A88:  A1 71 2F                     mov      ax, word ptr [0x2f71]
008A8B:  48                           dec      ax
008A8C:  79 03                        jns      0x8a91
008A8E:  B8 B4 00                     mov      ax, 0xb4
008A91:  A3 71 2F                     mov      word ptr [0x2f71], ax
008A94:  E9 FE 00                     jmp      0x8b95
; -- non-contiguous block: next 0x008aac --
008AAC:  FE 06 DF 27                  inc      byte ptr [0x27df]
008AB0:  E9 E2 00                     jmp      0x8b95
008AB3:  3C 02                        cmp      al, 2
008AB5:  75 29                        jne      0x8ae0
008AB7:  A1 69 2F                     mov      ax, word ptr [0x2f69]
008ABA:  3D 20 4E                     cmp      ax, 0x4e20
008ABD:  77 0F                        ja       0x8ace
008ABF:  03 06 6B 2F                  add      ax, word ptr [0x2f6b]
008AC3:  83 06 6B 2F 64               add      word ptr [0x2f6b], 0x64
008AC8:  A3 69 2F                     mov      word ptr [0x2f69], ax
008ACB:  E9 C7 00                     jmp      0x8b95
008ACE:  B8 15 00                     mov      ax, 0x15
008AD1:  BB 1F 00                     mov      bx, 0x1f
008AD4:  9A B0 12 99 02               lcall    0x299, 0x12b0
008AD9:  FE 06 DF 27                  inc      byte ptr [0x27df]
008ADD:  E9 B5 00                     jmp      0x8b95
008AE0:  3C 03                        cmp      al, 3
008AE2:  75 47                        jne      0x8b2b
008AE4:  C7 06 32 0A FF FF            mov      word ptr [0xa32], 0xffff
008AEA:  B8 04 00                     mov      ax, 4
008AED:  9A 41 12 99 02               lcall    0x299, 0x1241
008AF2:  C7 06 69 2F 20 4E            mov      word ptr [0x2f69], 0x4e20
008AF8:  C7 06 71 2F 00 00            mov      word ptr [0x2f71], 0
008AFE:  C7 06 65 2F 10 27            mov      word ptr [0x2f65], 0x2710
008B04:  BE 22 1F                     mov      si, 0x1f22
008B07:  A1 20 1F                     mov      ax, word ptr [0x1f20]
008B0A:  83 E0 07                     and      ax, 7
008B0D:  C1 E0 04                     shl      ax, 4
008B10:  03 F0                        add      si, ax
008B12:  FF 06 20 1F                  inc      word ptr [0x1f20]
008B16:  BF 06 21                     mov      di, 0x2106
008B19:  AC                           lodsb    al, byte ptr [si]
008B1A:  AA                           stosb    byte ptr es:[di], al
008B1B:  0A C0                        or       al, al
008B1D:  75 FA                        jne      0x8b19
008B1F:  C7 06 88 67 06 00            mov      word ptr [0x6788], 6
008B25:  FE 06 DF 27                  inc      byte ptr [0x27df]
008B29:  EB 7D                        jmp      0x8ba8
008B2B:  3C 04                        cmp      al, 4
008B2D:  75 2D                        jne      0x8b5c
008B2F:  9A 00 00 71 09               lcall    0x971, 0
008B34:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
008B39:  75 6D                        jne      0x8ba8
008B3B:  C7 06 32 0A 00 00            mov      word ptr [0xa32], 0
008B41:  B8 04 00                     mov      ax, 4
008B44:  9A 41 12 99 02               lcall    0x299, 0x1241
008B49:  0E                           push     cs
008B4A:  E8 49 01                     call     0x8c96
008B4D:  E8 4D 0A                     call     0x959d
008B50:  FE 06 DF 27                  inc      byte ptr [0x27df]
008B54:  C7 06 69 2F 30 75            mov      word ptr [0x2f69], 0x7530
008B5A:  EB 4C                        jmp      0x8ba8
008B5C:  A1 69 2F                     mov      ax, word ptr [0x2f69]
008B5F:  8B D8                        mov      bx, ax
008B61:  F7 DB                        neg      bx
008B63:  C1 EB 02                     shr      bx, 2
008B66:  74 07                        je       0x8b6f
008B68:  03 C3                        add      ax, bx
008B6A:  A3 69 2F                     mov      word ptr [0x2f69], ax
008B6D:  EB 26                        jmp      0x8b95
008B6F:  C7 06 6B 2F 10 00            mov      word ptr [0x2f6b], 0x10
008B75:  C7 06 69 2F 00 00            mov      word ptr [0x2f69], 0
008B7B:  C6 06 DA 27 00               mov      byte ptr [0x27da], 0
008B80:  C6 06 DF 27 00               mov      byte ptr [0x27df], 0
008B85:  83 26 93 27 FB               and      word ptr [0x2793], 0xfffb
008B8A:  E8 10 0A                     call     0x959d
008B8D:  C7 06 32 0A 01 00            mov      word ptr [0xa32], 1
008B93:  EB 13                        jmp      0x8ba8
008B95:  33 C0                        xor      ax, ax
008B97:  9A EB 0D 99 02               lcall    0x299, 0xdeb
008B9C:  0E                           push     cs
008B9D:  E8 19 0D                     call     0x98b9
008BA0:  0E                           push     cs
008BA1:  E8 6C 0E                     call     0x9a10
008BA4:  0E                           push     cs
008BA5:  E8 F0 0F                     call     0x9b98
008BA8:  5D                           pop      bp
008BA9:  5B                           pop      bx
008BAA:  C3                           ret     
