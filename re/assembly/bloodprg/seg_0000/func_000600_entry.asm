; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000600
; seg_off: 0000:0000
; group: seg_0000
; provenance: mz_entry, recursive_graph
; label: entry
; label_comment: MZ entry point; sets DS/SS=0x0ce2 SP=0x7e78
; byte_count: 241
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0x6e1:1, jmp 0x6ec:1
; direct_callees: 0x0006f1, 0x00079c, 0x0007ea, 0x00099f, 0x000a99, 0x000b32, 0x000b42, 0x000bff, 0x000c26, 0x000cc0, 0x000ccb, 0x000cef, 0x000d4a, 0x000d61
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_0000/func_000600_entry.cpp
; routine_bytes_sha256: cfdfc610f110ef623fe97e8ea7355a154add5103fac5b60e379aa72d78d35b08

000600:  B8 E2 0C                     mov      ax, 0xce2
000603:  8E D8                        mov      ds, ax
000605:  FA                           cli     
000606:  8E D0                        mov      ss, ax
000608:  BC 78 7E                     mov      sp, 0x7e78
00060B:  FB                           sti     
00060C:  0E                           push     cs
00060D:  E8 BB 06                     call     0xccb
000610:  0B C0                        or       ax, ax
000612:  75 0A                        jne      0x61e
000614:  BE 00 00                     mov      si, 0
000617:  0E                           push     cs
000618:  E8 46 07                     call     0xd61
00061B:  E9 CE 00                     jmp      0x6ec
00061E:  8C D8                        mov      ax, ds
000620:  8E E8                        mov      gs, ax
000622:  B8 BF 0B                     mov      ax, 0xbbf
000625:  8E E0                        mov      fs, ax
000627:  66 33 FF                     xor      edi, edi
00062A:  66 33 F6                     xor      esi, esi
00062D:  66 33 ED                     xor      ebp, ebp
000630:  66 33 DB                     xor      ebx, ebx
000633:  8B C4                        mov      ax, sp
000635:  83 C0 0F                     add      ax, 0xf
000638:  C1 E8 04                     shr      ax, 4
00063B:  8C D3                        mov      bx, ss
00063D:  03 C3                        add      ax, bx
00063F:  8C C3                        mov      bx, es
000641:  2B C3                        sub      ax, bx
000643:  93                           xchg     bx, ax
000644:  B8 00 4A                     mov      ax, 0x4a00
000647:  CD 21                        int      0x21
000649:  B8 00 48                     mov      ax, 0x4800
00064C:  BB FF FF                     mov      bx, 0xffff
00064F:  CD 21                        int      0x21
000651:  66 33 C0                     xor      eax, eax
000654:  8B C3                        mov      ax, bx
000656:  66 C1 E0 04                  shl      eax, 4
00065A:  66 A3 46 0A                  mov      dword ptr [0xa46], eax
00065E:  66 3D 70 88 07 00            cmp      eax, 0x78870
000664:  73 09                        jae      0x66f
000666:  BE 0E 00                     mov      si, 0xe
000669:  0E                           push     cs
00066A:  E8 F4 06                     call     0xd61
00066D:  EB 72                        jmp      0x6e1
00066F:  B8 00 48                     mov      ax, 0x4800
000672:  CD 21                        int      0x21
000674:  C7 06 42 0A 00 00            mov      word ptr [0xa42], 0
00067A:  A3 44 0A                     mov      word ptr [0xa44], ax
00067D:  A3 6A 0A                     mov      word ptr [0xa6a], ax
000680:  B8 29 0B                     mov      ax, 0xb29
000683:  A3 F0 0A                     mov      word ptr [0xaf0], ax
000686:  E8 68 00                     call     0x6f1
000689:  0E                           push     cs
00068A:  E8 62 06                     call     0xcef
00068D:  9A F3 0A CE 01               lcall    0x1ce, 0xaf3
000692:  0E                           push     cs
000693:  E8 69 05                     call     0xbff
000696:  0E                           push     cs
000697:  E8 02 01                     call     0x79c
00069A:  0E                           push     cs
00069B:  E8 88 05                     call     0xc26
00069E:  E8 91 04                     call     0xb32
0006A1:  B8 00 00                     mov      ax, 0
0006A4:  BB B8 0B                     mov      bx, 0xbb8
0006A7:  B9 00 00                     mov      cx, 0
0006AA:  BA C8 00                     mov      dx, 0xc8
0006AD:  0E                           push     cs
0006AE:  E8 99 06                     call     0xd4a
0006B1:  B9 D0 02                     mov      cx, 0x2d0
0006B4:  BA 96 00                     mov      dx, 0x96
0006B7:  B8 04 00                     mov      ax, 4
0006BA:  CD 33                        int      0x33
0006BC:  0E                           push     cs
0006BD:  E8 82 04                     call     0xb42
0006C0:  0E                           push     cs
0006C1:  E8 DB 02                     call     0x99f
0006C4:  B0 B6                        mov      al, 0xb6
0006C6:  E6 43                        out      0x43, al
0006C8:  B0 9C                        mov      al, 0x9c
0006CA:  E6 42                        out      0x42, al
0006CC:  B0 2E                        mov      al, 0x2e
0006CE:  E6 42                        out      0x42, al
0006D0:  9A 00 00 8B 00               lcall    0x8b, 0
0006D5:  0E                           push     cs
0006D6:  E8 C0 03                     call     0xa99
0006D9:  0E                           push     cs
0006DA:  E8 0D 01                     call     0x7ea
0006DD:  0E                           push     cs
0006DE:  E8 DF 05                     call     0xcc0
0006E1:  65 A1 44 0A                  mov      ax, word ptr gs:[0xa44]
0006E5:  8E C0                        mov      es, ax
0006E7:  B8 00 49                     mov      ax, 0x4900
0006EA:  CD 21                        int      0x21
0006EC:  B8 00 4C                     mov      ax, 0x4c00
0006EF:  CD 21                        int      0x21
