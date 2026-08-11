; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00933a
; seg_off: 071e:1b5a
; group: seg_071e
; provenance: recursive_graph
; label: back_buffer_copy_from
; label_comment: blit helper (10 calls): les di,gs:[0x5229] (linear back-buffer dest); lds si,gs:[0xabc] (source surface); copies from the gs:0xabc source into the back buffer. A surface->backbuffer copy
; byte_count: 42
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_071e/func_00933a_back_buffer_copy_from.cpp
; routine_bytes_sha256: 0f0d19e171bb60749bf5523468b18aa2a731b735356689d76ba4f520f8161fc3

00933A:  06                           push     es
00933B:  57                           push     di
00933C:  1E                           push     ds
00933D:  56                           push     si
00933E:  51                           push     cx
00933F:  50                           push     ax
009340:  65 C4 3E 29 52               les      di, ptr gs:[0x5229]
009345:  65 C5 36 BC 0A               lds      si, ptr gs:[0xabc]
00934A:  8B C1                        mov      ax, cx
00934C:  86 C4                        xchg     ah, al
00934E:  C1 E1 06                     shl      cx, 6
009351:  03 C1                        add      ax, cx
009353:  8B F8                        mov      di, ax
009355:  03 FB                        add      di, bx
009357:  8B F7                        mov      si, di
009359:  8B CA                        mov      cx, dx
00935B:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00935D:  58                           pop      ax
00935E:  59                           pop      cx
00935F:  5E                           pop      si
009360:  1F                           pop      ds
009361:  5F                           pop      di
009362:  07                           pop      es
009363:  C3                           ret     
