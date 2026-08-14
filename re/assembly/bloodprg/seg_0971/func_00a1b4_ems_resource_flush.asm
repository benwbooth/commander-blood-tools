; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a1b4
; seg_off: 0971:04a4
; group: seg_0971
; provenance: recursive_graph
; label: ems_resource_flush
; label_comment: service one presentation-queue frame. Reject an unavailable nonbanked source, refill and retry until an entry activates, gate it on the audio/software clock, optionally apply palette records, present and consume the entry, then enter the shared 0x00a1f3 latch/refill return tail. The CALL 0x00a1f3 at 0x00a1d1 supplies an extra return word and corrupts that tail's parent-frame unwind; direct execution records it as a dead/invalid-state edge, while the natural C recovery performs the intended helper call followed by return.
; byte_count: 88
; boundary: cfg_blocks_12_terminals_2
; terminal: jmp 0xa1bc:1, ret:1
; direct_callees: 0x00a1f3, 0x00a20c, 0x00a240, 0x00a2ab, 0x00a3d0, 0x00a41a, 0x00a778
; indirect_calls: 0
; routine_bytes_sha256: 40c7a6d363d6cebf8c0a8bf5918ec51f867652c93f859d5663c1ee419ee36a97

00A1B4:  1E                           push     ds
00A1B5:  56                           push     si
00A1B6:  06                           push     es
00A1B7:  57                           push     di
00A1B8:  53                           push     bx
00A1B9:  51                           push     cx
00A1BA:  52                           push     dx
00A1BB:  55                           push     bp
00A1BC:  F6 06 BC 0D 01               test     byte ptr [0xdbc], 1
00A1C1:  75 11                        jne      0xa1d4
00A1C3:  83 3E 5B 0D 00               cmp      word ptr [0xd5b], 0
00A1C8:  74 34                        je       0xa1fe
00A1CA:  80 3E 76 0D 00               cmp      byte ptr [0xd76], 0
00A1CF:  79 03                        jns      0xa1d4
00A1D1:  E8 1F 00                     call     0xa1f3
00A1D4:  E8 35 00                     call     0xa20c
00A1D7:  73 05                        jae      0xa1de
00A1D9:  E8 CF 00                     call     0xa2ab
00A1DC:  EB DE                        jmp      0xa1bc
00A1DE:  E8 5F 00                     call     0xa240
00A1E1:  72 10                        jb       0xa1f3
00A1E3:  A1 9E 0D                     mov      ax, word ptr [0xd9e]
00A1E6:  40                           inc      ax
00A1E7:  74 03                        je       0xa1ec
00A1E9:  E8 8C 05                     call     0xa778
00A1EC:  0E                           push     cs
00A1ED:  E8 2A 02                     call     0xa41a
00A1F0:  E8 DD 01                     call     0xa3d0
; -- non-contiguous block: next 0x00a1fe --
00A1FE:  C6 06 AC 0D 00               mov      byte ptr [0xdac], 0
00A203:  5D                           pop      bp
00A204:  5A                           pop      dx
00A205:  59                           pop      cx
00A206:  5B                           pop      bx
00A207:  5F                           pop      di
00A208:  07                           pop      es
00A209:  5E                           pop      si
00A20A:  1F                           pop      ds
00A20B:  C3                           ret     
