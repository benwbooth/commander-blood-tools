; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a291
; seg_off: 0971:059b
; group: seg_0971
; provenance: recursive_graph, backward_cfg_completion
; label: list_d8c_refill
; label_comment: refill the D8C ring queue. Consume pending bytes in chunks, cap ordinary resources at the next 0x800 boundary plus 0x800, read new extent headers when a chunk ends, and on source exhaustion either finish the queue or roll to the next resource range and optionally synthesize four 0x6d6d link records.
; shared_tail_entries: 0x00a2dd, 0x00a664
; byte_count: 253
; boundary: cfg_blocks_26_terminals_4
; terminal: jmp 0xa664:1, ret:3
; direct_callees: 0x009f80, 0x009fa2, 0x00a141, 0x00a38e, 0x00a3ad, 0x00a622, 0x00a734, 0x00a744, 0x00a7e6
; indirect_calls: 0
; routine_bytes_sha256: 4d36349869310c376a92e5443f1730b71d5d93f788e059fdcef6ca6f4954658b

; -- backward block reached when the current pending extent is exhausted --
00A291:  A1 62 0D                     mov      ax, word ptr [0xd62]
00A294:  3B 06 66 0D                  cmp      ax, word ptr [0xd66]
00A298:  74 58                        je       0xa2f2
00A29A:  A1 88 0D                     mov      ax, word ptr [0xd88]
00A29D:  0B 06 8A 0D                  or       ax, word ptr [0xd8a]
00A2A1:  74 4F                        je       0xa2f2
00A2A3:  E8 7C 03                     call     0xa622
00A2A6:  72 2D                        jb       0xa2d5
00A2A8:  E8 E3 00                     call     0xa38e

00A2AB:  8B 0E A0 0D                  mov      cx, word ptr [0xda0]
00A2AF:  E3 E0                        jcxz     0xa291
00A2B1:  80 3E 76 0D 00               cmp      byte ptr [0xd76], 0
00A2B6:  78 11                        js       0xa2c9
00A2B8:  A1 84 0D                     mov      ax, word ptr [0xd84]
00A2BB:  F7 D8                        neg      ax
00A2BD:  25 FF 07                     and      ax, 0x7ff
00A2C0:  80 C4 08                     add      ah, 8
00A2C3:  3B C1                        cmp      ax, cx
00A2C5:  73 02                        jae      0xa2c9
00A2C7:  8B C8                        mov      cx, ax
00A2C9:  E8 E1 00                     call     0xa3ad
00A2CC:  72 07                        jb       0xa2d5
00A2CE:  29 0E A0 0D                  sub      word ptr [0xda0], cx
00A2D2:  E9 8F 03                     jmp      0xa664
00A2D5:  C3                           ret     

; -- forward exhaustion/resource-rollover blocks --
00A2D6:  A3 80 0D                     mov      word ptr [0xd80], ax
00A2D9:  E8 C6 FC                     call     0x9fa2
00A2DC:  C3                           ret
; -- shared tail entry: presentation_queue_finish 0x00a2dd --
00A2DD:  80 0E 5F 0D 01               or       byte ptr [0xd5f], 1
00A2E2:  83 3E 9A 0D 00               cmp      word ptr [0xd9a], 0
00A2E7:  75 08                        jne      0xa2f1
00A2E9:  80 0E 5F 0D 02               or       byte ptr [0xd5f], 2
00A2EE:  E8 50 FE                     call     0xa141
00A2F1:  C3                           ret
00A2F2:  F6 06 76 0D 01               test     byte ptr [0xd76], 1
00A2F7:  74 E4                        je       0xa2dd
00A2F9:  B9 00 10                     mov      cx, 0x1000
00A2FC:  E8 AE 00                     call     0xa3ad
00A2FF:  72 D4                        jb       0xa2d5
00A301:  A1 62 0D                     mov      ax, word ptr [0xd62]
00A304:  E8 3D 04                     call     0xa744
00A307:  A3 64 0D                     mov      word ptr [0xd64], ax
00A30A:  C6 06 AC 0D 00               mov      byte ptr [0xdac], 0
00A30F:  A1 82 0D                     mov      ax, word ptr [0xd82]
00A312:  3B 06 80 0D                  cmp      ax, word ptr [0xd80]
00A316:  74 1B                        je       0xa333
00A318:  E8 65 FC                     call     0x9f80
00A31B:  F6 07 08                     test     byte ptr [bx], 8
00A31E:  74 B6                        je       0xa2d6
00A320:  83 7F FA 00                  cmp      word ptr [bx - 6], 0
00A324:  74 B0                        je       0xa2d6
00A326:  A3 80 0D                     mov      word ptr [0xd80], ax
00A329:  8D 77 F8                     lea      si, [bx - 8]
00A32C:  BF 6E 0D                     mov      di, 0xd6e
00A32F:  E8 B4 04                     call     0xa7e6
00A332:  A4                           movsb    byte ptr es:[di], byte ptr [si]
00A333:  A1 72 0D                     mov      ax, word ptr [0xd72]
00A336:  A3 88 0D                     mov      word ptr [0xd88], ax
00A339:  A1 74 0D                     mov      ax, word ptr [0xd74]
00A33C:  A3 8A 0D                     mov      word ptr [0xd8a], ax
00A33F:  A1 6E 0D                     mov      ax, word ptr [0xd6e]
00A342:  A3 84 0D                     mov      word ptr [0xd84], ax
00A345:  A1 70 0D                     mov      ax, word ptr [0xd70]
00A348:  A3 86 0D                     mov      word ptr [0xd86], ax
00A34B:  F6 06 76 0D 04               test     byte ptr [0xd76], 4
00A350:  74 39                        je       0xa38b
00A352:  B9 04 00                     mov      cx, 4
00A355:  C4 3E 8C 0D                  les      di, ptr [0xd8c]
00A359:  B8 02 00                     mov      ax, 2
00A35C:  E8 D5 03                     call     0xa734
00A35F:  B8 0A 00                     mov      ax, 0xa
00A362:  AB                           stosw    word ptr es:[di], ax
00A363:  8B F7                        mov      si, di
00A365:  E8 26 00                     call     0xa38e
00A368:  26 FF 76 00                  push     word ptr es:[bp]
00A36C:  06                           push     es
00A36D:  C4 3E 8C 0D                  les      di, ptr [0xd8c]
00A371:  B8 6D 6D                     mov      ax, 0x6d6d
00A374:  AB                           stosw    word ptr es:[di], ax
00A375:  8B C5                        mov      ax, bp
00A377:  AB                           stosw    word ptr es:[di], ax
00A378:  58                           pop      ax
00A379:  AB                           stosw    word ptr es:[di], ax
00A37A:  58                           pop      ax
00A37B:  AB                           stosw    word ptr es:[di], ax
00A37C:  03 E8                        add      bp, ax
00A37E:  B8 08 00                     mov      ax, 8
00A381:  E8 B0 03                     call     0xa734
00A384:  E2 CF                        loop     0xa355
00A386:  C6 06 AC 0D 80               mov      byte ptr [0xdac], 0x80
00A38B:  E9 03 FF                     jmp      0xa291
