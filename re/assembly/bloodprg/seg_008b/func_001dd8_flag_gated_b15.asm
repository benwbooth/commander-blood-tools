; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001dd8
; seg_off: 008b:0f28
; group: seg_008b
; provenance: recursive_graph
; label: flag_gated_b15
; label_comment: flag-gated routine: si=0x273b; al=[0xb15]; test/branch. Gates on the 0xb15 state byte (a mode/enable flag)
; byte_count: 133
; boundary: cfg_blocks_15_terminals_3
; terminal: jmp 0x1e27:1, jmp 0x1e55:1, ret:1
; direct_callees: none
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_008b/func_001dd8_flag_gated_b15.cpp
; routine_bytes_sha256: 3a1706e8bcaac3b07eb0b174ee8eb810ee35f9798ad785e5d49db82a04889a98

001DD8:  50                           push     ax
001DD9:  53                           push     bx
001DDA:  51                           push     cx
001DDB:  52                           push     dx
001DDC:  57                           push     di
001DDD:  56                           push     si
001DDE:  55                           push     bp
001DDF:  BE 3B 27                     mov      si, 0x273b
001DE2:  A0 15 0B                     mov      al, byte ptr [0xb15]
001DE5:  0A C0                        or       al, al
001DE7:  74 3E                        je       0x1e27
001DE9:  8B 1E 2E 27                  mov      bx, word ptr [0x272e]
001DED:  3C 0D                        cmp      al, 0xd
001DEF:  75 11                        jne      0x1e02
001DF1:  0B DB                        or       bx, bx
001DF3:  74 32                        je       0x1e27
001DF5:  B9 04 00                     mov      cx, 4
001DF8:  8B 3E 34 27                  mov      di, word ptr [0x2734]
001DFC:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
001DFF:  F9                           stc     
001E00:  EB 53                        jmp      0x1e55
001E02:  3C 30                        cmp      al, 0x30
001E04:  72 15                        jb       0x1e1b
001E06:  3C 39                        cmp      al, 0x39
001E08:  76 08                        jbe      0x1e12
001E0A:  3C 61                        cmp      al, 0x61
001E0C:  72 0D                        jb       0x1e1b
001E0E:  3C 7A                        cmp      al, 0x7a
001E10:  77 09                        ja       0x1e1b
001E12:  80 FB 0E                     cmp      bl, 0xe
001E15:  74 10                        je       0x1e27
001E17:  88 00                        mov      byte ptr [bx + si], al
001E19:  EB 0C                        jmp      0x1e27
001E1B:  3C 08                        cmp      al, 8
001E1D:  75 08                        jne      0x1e27
001E1F:  0B DB                        or       bx, bx
001E21:  74 04                        je       0x1e27
001E23:  4B                           dec      bx
001E24:  C6 00 20                     mov      byte ptr [bx + si], 0x20
001E27:  A1 32 27                     mov      ax, word ptr [0x2732]
001E2A:  BA 0B 00                     mov      dx, 0xb
001E2D:  F6 E2                        mul      dl
001E2F:  83 C0 27                     add      ax, 0x27
001E32:  8B C8                        mov      cx, ax
001E34:  8B 1E AB 2A                  mov      bx, word ptr [0x2aab]
001E38:  B8 E8 00                     mov      ax, 0xe8
001E3B:  BD 0A 00                     mov      bp, 0xa
001E3E:  8B 16 AF 2A                  mov      dx, word ptr [0x2aaf]
001E42:  9A DC 0C 99 02               lcall    0x299, 0xcdc
001E47:  41                           inc      cx
001E48:  8B D1                        mov      dx, cx
001E4A:  83 C3 0A                     add      bx, 0xa
001E4D:  B0 EF                        mov      al, 0xef
001E4F:  9A 76 01 99 02               lcall    0x299, 0x176
001E54:  F8                           clc     
001E55:  5D                           pop      bp
001E56:  5E                           pop      si
001E57:  5F                           pop      di
001E58:  5A                           pop      dx
001E59:  59                           pop      cx
001E5A:  5B                           pop      bx
001E5B:  58                           pop      ax
001E5C:  C3                           ret     
