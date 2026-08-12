; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a642
; seg_off: 0971:0932
; group: seg_0971
; provenance: recursive_graph
; label: banked_list_load
; label_comment: load the gs:0xd8c banked list: call list_d8c_init 0xa757; call list_d8c_read 0xa622; di=[0x5233] (buffer end). After placing the first extent word at the high end of the ring buffer, execution falls through ems_paged_read 0xa664 and then queue_d8c_enqueue 0xa734 before the common RET.
; shared_tail_entries: 0x00a664, 0x00a734
; byte_count: 252
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x00a622, 0x00a757
; indirect_calls: 0
; routine_bytes_sha256: ad608c6fb80044aeec02c44132c4bd973b66c4c1e6431349799532debce68133

00A642:  0E                           push     cs
00A643:  E8 11 01                     call     0xa757
00A646:  E8 D9 FF                     call     0xa622
00A649:  0F 82 F0 00                  jb       0xa73d
00A64D:  8B 3E 33 52                  mov      di, word ptr [0x5233]
00A651:  2B F8                        sub      di, ax
00A653:  83 EF 02                     sub      di, 2
00A656:  89 3E 90 0D                  mov      word ptr [0xd90], di
00A65A:  AB                           stosw    word ptr es:[di], ax
00A65B:  89 3E 8C 0D                  mov      word ptr [0xd8c], di
00A65F:  8B C8                        mov      cx, ax
00A661:  83 E9 02                     sub      cx, 2
00A664:  F6 06 BC 0D 01               test     byte ptr [0xdbc], 1
00A669:  0F 84 8F 00                  je       0xa6fc
00A66D:  83 3E 58 0A FF               cmp      word ptr [0xa58], -1
00A672:  74 41                        je       0xa6b5
00A674:  1E                           push     ds
00A675:  56                           push     si
00A676:  06                           push     es
00A677:  57                           push     di
00A678:  66 A1 84 0D                  mov      eax, dword ptr [0xd84]
00A67C:  8B F0                        mov      si, ax
00A67E:  81 E6 FF 3F                  and      si, 0x3fff
00A682:  66 C1 E8 0E                  shr      eax, 0xe
00A686:  8B D8                        mov      bx, ax
00A688:  51                           push     cx
00A689:  B9 04 00                     mov      cx, 4
00A68C:  8B 16 58 0A                  mov      dx, word ptr [0xa58]
00A690:  32 C0                        xor      al, al
00A692:  B4 44                        mov      ah, 0x44
00A694:  CD 67                        int      0x67
00A696:  FE C0                        inc      al
00A698:  43                           inc      bx
00A699:  E2 F7                        loop     0xa692
00A69B:  59                           pop      cx
00A69C:  66 0F B7 C1                  movzx    eax, cx
00A6A0:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
00A6A5:  65 C4 3E 8C 0D               les      di, ptr gs:[0xd8c]
00A6AA:  9A 93 0B CE 01               lcall    0x1ce, 0xb93
00A6AF:  5F                           pop      di
00A6B0:  07                           pop      es
00A6B1:  5E                           pop      si
00A6B2:  1F                           pop      ds
00A6B3:  EB 6D                        jmp      0xa722
00A6B5:  83 3E 56 0A FF               cmp      word ptr [0xa56], -1
00A6BA:  74 40                        je       0xa6fc
00A6BC:  06                           push     es
00A6BD:  1E                           push     ds
00A6BE:  57                           push     di
00A6BF:  56                           push     si
00A6C0:  8C E8                        mov      ax, gs
00A6C2:  8E C0                        mov      es, ax
00A6C4:  8E D8                        mov      ds, ax
00A6C6:  BF 6C 0A                     mov      di, 0xa6c
00A6C9:  8B F7                        mov      si, di
00A6CB:  66 0F B7 C1                  movzx    eax, cx
00A6CF:  A8 01                        test     al, 1
00A6D1:  74 02                        je       0xa6d5
00A6D3:  66 40                        inc      eax
00A6D5:  66 AB                        stosd    dword ptr es:[di], eax
00A6D7:  A1 56 0A                     mov      ax, word ptr [0xa56]
00A6DA:  AB                           stosw    word ptr es:[di], ax
00A6DB:  66 A1 84 0D                  mov      eax, dword ptr [0xd84]
00A6DF:  66 AB                        stosd    dword ptr es:[di], eax
00A6E1:  33 C0                        xor      ax, ax
00A6E3:  AB                           stosw    word ptr es:[di], ax
00A6E4:  66 A1 8C 0D                  mov      eax, dword ptr [0xd8c]
00A6E8:  66 AB                        stosd    dword ptr es:[di], eax
00A6EA:  66 B8 00 0B 00 00            mov      eax, 0xb00
00A6F0:  FF 1E 4A 0A                  lcall    [0xa4a]
00A6F4:  8B C1                        mov      ax, cx
00A6F6:  5E                           pop      si
00A6F7:  5F                           pop      di
00A6F8:  1F                           pop      ds
00A6F9:  07                           pop      es
00A6FA:  EB 26                        jmp      0xa722
00A6FC:  8B 1E 5B 0D                  mov      bx, word ptr [0xd5b]
00A700:  83 FB 01                     cmp      bx, 1
00A703:  72 38                        jb       0xa73d
00A705:  51                           push     cx
00A706:  8B 0E 86 0D                  mov      cx, word ptr [0xd86]
00A70A:  8B 16 84 0D                  mov      dx, word ptr [0xd84]
00A70E:  B8 00 42                     mov      ax, 0x4200
00A711:  CD 21                        int      0x21
00A713:  59                           pop      cx
00A714:  1E                           push     ds
00A715:  C5 16 8C 0D                  lds      dx, ptr [0xd8c]
00A719:  B4 3F                        mov      ah, 0x3f
00A71B:  CD 21                        int      0x21
00A71D:  1F                           pop      ds
00A71E:  3B C1                        cmp      ax, cx
00A720:  72 E3                        jb       0xa705
00A722:  29 06 88 0D                  sub      word ptr [0xd88], ax
00A726:  83 1E 8A 0D 00               sbb      word ptr [0xd8a], 0
00A72B:  01 06 84 0D                  add      word ptr [0xd84], ax
00A72F:  83 16 86 0D 00               adc      word ptr [0xd86], 0
00A734:  01 06 8C 0D                  add      word ptr [0xd8c], ax
00A738:  01 06 9A 0D                  add      word ptr [0xd9a], ax
00A73C:  F8                           clc
00A73D:  C3                           ret     
