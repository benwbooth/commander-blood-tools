; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00aabc
; seg_off: 0971:0dac
; group: seg_0971
; provenance: recursive_graph
; label: rle_decode_step
; label_comment: RLE decode step (2 calls): ah=0; al=[bx] control byte; if negative (js) it's a run (al+=al, cl=[bx+1] run length). A run-length decompression primitive (same negative-control convention as the sprite blit)
; byte_count: 105
; boundary: cfg_blocks_15_terminals_4
; terminal: ret:4
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00aabc_rle_decode_step.cpp
; routine_bytes_sha256: 6f9aba91cf84930a552caddcbe3511006f7b3c6cca5ee86f53b57935f6816775

00AABC:  32 E4                        xor      ah, ah
00AABE:  8B DE                        mov      bx, si
00AAC0:  33 C9                        xor      cx, cx
00AAC2:  8A 07                        mov      al, byte ptr [bx]
00AAC4:  0A C0                        or       al, al
00AAC6:  79 22                        jns      0xaaea
00AAC8:  02 C0                        add      al, al
00AACA:  8A 4F 01                     mov      cl, byte ptr [bx + 1]
00AACD:  8A D1                        mov      dl, cl
00AACF:  C0 E9 05                     shr      cl, 5
00AAD2:  14 00                        adc      al, 0
00AAD4:  80 C1 02                     add      cl, 2
00AAD7:  F7 D0                        not      ax
00AAD9:  8B F7                        mov      si, di
00AADB:  03 F0                        add      si, ax
00AADD:  32 E4                        xor      ah, ah
00AADF:  F3 26 A4                     rep movsb byte ptr es:[di], byte ptr es:[si]
00AAE2:  83 C3 02                     add      bx, 2
00AAE5:  3B FD                        cmp      di, bp
00AAE7:  72 0C                        jb       0xaaf5
00AAE9:  C3                           ret     
00AAEA:  74 02                        je       0xaaee
00AAEC:  04 0C                        add      al, 0xc
00AAEE:  AA                           stosb    byte ptr es:[di], al
00AAEF:  43                           inc      bx
00AAF0:  3B FD                        cmp      di, bp
00AAF2:  72 CE                        jb       0xaac2
00AAF4:  C3                           ret     
00AAF5:  8A 07                        mov      al, byte ptr [bx]
00AAF7:  0A C0                        or       al, al
00AAF9:  79 1F                        jns      0xab1a
00AAFB:  8A CA                        mov      cl, dl
00AAFD:  80 E1 0F                     and      cl, 0xf
00AB00:  02 C0                        add      al, al
00AB02:  D0 E9                        shr      cl, 1
00AB04:  14 00                        adc      al, 0
00AB06:  80 C1 02                     add      cl, 2
00AB09:  F7 D0                        not      ax
00AB0B:  8B F7                        mov      si, di
00AB0D:  03 F0                        add      si, ax
00AB0F:  32 E4                        xor      ah, ah
00AB11:  F3 26 A4                     rep movsb byte ptr es:[di], byte ptr es:[si]
00AB14:  43                           inc      bx
00AB15:  3B FD                        cmp      di, bp
00AB17:  72 A9                        jb       0xaac2
00AB19:  C3                           ret     
00AB1A:  74 02                        je       0xab1e
00AB1C:  04 0C                        add      al, 0xc
00AB1E:  AA                           stosb    byte ptr es:[di], al
00AB1F:  43                           inc      bx
00AB20:  3B FD                        cmp      di, bp
00AB22:  72 D1                        jb       0xaaf5
00AB24:  C3                           ret     
