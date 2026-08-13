; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a82c
; seg_off: 0971:0b1c
; group: seg_0971
; provenance: recursive_graph
; label: resource_payload_decode_dispatch
; label_comment: Clears destination-offset bit 9, sums six source-header bytes modulo 256, dispatches checksum 0xAB to the record-stream decoder and checksum 0xAD to the alternate-segment decoder, and otherwise leaves the source unchanged.
; byte_count: 59
; boundary: cfg_blocks_5_terminals_2
; terminal: ret:2
; direct_callees: 0x00a867, 0x00a914
; indirect_calls: 0
; routine_bytes_sha256: 75ac345da65c1258183340faaf1505164d8b4b53fc2f2b1288a95d0b31c730ee

00A82C:  81 E7 FF FD                  and      di, 0xfdff
00A830:  51                           push     cx
00A831:  57                           push     di
00A832:  51                           push     cx
00A833:  56                           push     si
00A834:  B9 06 00                     mov      cx, 6
00A837:  33 C0                        xor      ax, ax
00A839:  AC                           lodsb    al, byte ptr [si]
00A83A:  02 E0                        add      ah, al
00A83C:  E2 FB                        loop     0xa839
00A83E:  5E                           pop      si
00A83F:  59                           pop      cx
00A840:  80 FC AD                     cmp      ah, 0xad
00A843:  74 0D                        je       0xa852
00A845:  80 FC AB                     cmp      ah, 0xab
00A848:  75 05                        jne      0xa84f
00A84A:  E8 1A 00                     call     0xa867
00A84D:  8B F7                        mov      si, di
00A84F:  5F                           pop      di
00A850:  59                           pop      cx
00A851:  C3                           ret     
00A852:  65 C7 06 A0 0A 03 00         mov      word ptr gs:[0xaa0], 3
00A859:  8E C5                        mov      es, bp
00A85B:  E8 B6 00                     call     0xa914
00A85E:  8C C0                        mov      ax, es
00A860:  8E D8                        mov      ds, ax
00A862:  33 F6                        xor      si, si
00A864:  5F                           pop      di
00A865:  59                           pop      cx
00A866:  C3                           ret     
