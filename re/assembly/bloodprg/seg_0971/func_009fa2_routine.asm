; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009fa2
; seg_off: 0971:0292
; group: seg_0971
; provenance: recursive_graph
; byte_count: 289
; boundary: cfg_blocks_19_terminals_1
; terminal: ret:1
; direct_callees: 0x009f80, 0x00a0c3, 0x00a622, 0x00a664
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_0971/func_009fa2_routine.cpp
; routine_bytes_sha256: 90d0d235c176541cf43021868d5a1436e0c5e83012bb90f7205eaabcc345a7eb

009FA2:  A1 80 0D                     mov      ax, word ptr [0xd80]
009FA5:  A3 82 0D                     mov      word ptr [0xd82], ax
009FA8:  E8 D5 FF                     call     0x9f80
009FAB:  A0 B1 1F                     mov      al, byte ptr [0x1fb1]
009FAE:  88 47 01                     mov      byte ptr [bx + 1], al
009FB1:  8B 07                        mov      ax, word ptr [bx]
009FB3:  A3 76 0D                     mov      word ptr [0xd76], ax
009FB6:  8D 57 02                     lea      dx, [bx + 2]
009FB9:  66 A1 52 0A                  mov      eax, dword ptr [0xa52]
009FBD:  66 A3 88 0D                  mov      dword ptr [0xd88], eax
009FC1:  66 33 C0                     xor      eax, eax
009FC4:  F6 06 BC 0D 01               test     byte ptr [0xdbc], 1
009FC9:  75 44                        jne      0xa00f
009FCB:  9A B3 03 CE 01               lcall    0x1ce, 0x3b3
009FD0:  89 1E 5B 0D                  mov      word ptr [0xd5b], bx
009FD4:  66 A1 8E 0A                  mov      eax, dword ptr [0xa8e]
009FD8:  66 A3 88 0D                  mov      dword ptr [0xd88], eax
009FDC:  66 A1 8A 0A                  mov      eax, dword ptr [0xa8a]
009FE0:  66 A3 84 0D                  mov      dword ptr [0xd84], eax
009FE4:  F6 06 E2 0A 01               test     byte ptr [0xae2], 1
009FE9:  75 2E                        jne      0xa019
009FEB:  06                           push     es
009FEC:  B8 00 2F                     mov      ax, 0x2f00
009FEF:  CD 21                        int      0x21
009FF1:  8B F3                        mov      si, bx
009FF3:  83 C6 1A                     add      si, 0x1a
009FF6:  33 C9                        xor      cx, cx
009FF8:  B8 00 4E                     mov      ax, 0x4e00
009FFB:  CD 21                        int      0x21
009FFD:  66 26 8B 04                  mov      eax, dword ptr es:[si]
00A001:  66 A3 88 0D                  mov      dword ptr [0xd88], eax
00A005:  07                           pop      es
00A006:  B8 00 3D                     mov      ax, 0x3d00
00A009:  CD 21                        int      0x21
00A00B:  0F 82 B1 00                  jb       0xa0c0
00A00F:  A3 5B 0D                     mov      word ptr [0xd5b], ax
00A012:  66 33 C0                     xor      eax, eax
00A015:  66 A3 84 0D                  mov      dword ptr [0xd84], eax
00A019:  FF 36 9A 0D                  push     word ptr [0xd9a]
00A01D:  FF 36 8C 0D                  push     word ptr [0xd8c]
00A021:  E8 FE 05                     call     0xa622
00A024:  72 1B                        jb       0xa041
00A026:  A3 AF 0D                     mov      word ptr [0xdaf], ax
00A029:  03 F0                        add      si, ax
00A02B:  72 06                        jb       0xa033
00A02D:  3B 36 33 52                  cmp      si, word ptr [0x5233]
00A031:  76 06                        jbe      0xa039
00A033:  C7 06 8C 0D 00 00            mov      word ptr [0xd8c], 0
00A039:  83 E8 02                     sub      ax, 2
00A03C:  8B C8                        mov      cx, ax
00A03E:  E8 23 06                     call     0xa664
00A041:  8F 06 8C 0D                  pop      word ptr [0xd8c]
00A045:  8F 06 9A 0D                  pop      word ptr [0xd9a]
00A049:  72 75                        jb       0xa0c0
00A04B:  C4 36 8C 0D                  les      si, ptr [0xd8c]
00A04F:  26 AD                        lodsw    ax, word ptr es:[si]
00A051:  03 C6                        add      ax, si
00A053:  72 06                        jb       0xa05b
00A055:  3B 06 33 52                  cmp      ax, word ptr [0x5233]
00A059:  76 02                        jbe      0xa05d
00A05B:  33 F6                        xor      si, si
00A05D:  C6 06 B7 0D FF               mov      byte ptr [0xdb7], 0xff
00A062:  E8 5E 00                     call     0xa0c3
00A065:  4E                           dec      si
00A066:  46                           inc      si
00A067:  26 80 3C FF                  cmp      byte ptr es:[si], 0xff
00A06B:  74 F9                        je       0xa066
00A06D:  33 DB                        xor      bx, bx
00A06F:  F6 06 76 0D 04               test     byte ptr [0xd76], 4
00A074:  74 02                        je       0xa078
00A076:  B3 10                        mov      bl, 0x10
00A078:  26 8B 08                     mov      cx, word ptr es:[bx + si]
00A07B:  26 8B 58 02                  mov      bx, word ptr es:[bx + si + 2]
00A07F:  A1 84 0D                     mov      ax, word ptr [0xd84]
00A082:  03 C1                        add      ax, cx
00A084:  A3 6E 0D                     mov      word ptr [0xd6e], ax
00A087:  A1 86 0D                     mov      ax, word ptr [0xd86]
00A08A:  13 C3                        adc      ax, bx
00A08C:  A3 70 0D                     mov      word ptr [0xd70], ax
00A08F:  A1 88 0D                     mov      ax, word ptr [0xd88]
00A092:  2B C1                        sub      ax, cx
00A094:  A3 72 0D                     mov      word ptr [0xd72], ax
00A097:  A1 8A 0D                     mov      ax, word ptr [0xd8a]
00A09A:  1B C3                        sbb      ax, bx
00A09C:  A3 74 0D                     mov      word ptr [0xd74], ax
00A09F:  8B 1E AF 0D                  mov      bx, word ptr [0xdaf]
00A0A3:  C1 E3 02                     shl      bx, 2
00A0A6:  66 26 8B 00                  mov      eax, dword ptr es:[bx + si]
00A0AA:  66 03 06 84 0D               add      eax, dword ptr [0xd84]
00A0AF:  66 A3 78 0D                  mov      dword ptr [0xd78], eax
00A0B3:  66 A1 88 0D                  mov      eax, dword ptr [0xd88]
00A0B7:  66 26 2B 00                  sub      eax, dword ptr es:[bx + si]
00A0BB:  66 A3 7C 0D                  mov      dword ptr [0xd7c], eax
00A0BF:  F8                           clc     
00A0C0:  66 58                        pop      eax
00A0C2:  C3                           ret     
