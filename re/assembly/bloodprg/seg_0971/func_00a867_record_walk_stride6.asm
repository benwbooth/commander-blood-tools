; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a867
; seg_off: 0971:0b57
; group: seg_0971
; provenance: recursive_graph
; label: resource_payload_decode_ab
; label_comment: LSB-first control-word decoder selected by the 0xAB six-byte header checksum. Supports literals, two-bit short matches, 13-bit long negative displacements, extended lengths, and a zero-length terminator; returns the consumed source cursor and decoded byte count.
; byte_count: 173
; boundary: cfg_blocks_17_terminals_4
; terminal: jmp 0xa8a0:3, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 68f21ed3bcbe8308592cb8c1c2773657b5066df70b3a5ce647b352863e477d56

00A867:  51                           push     cx
00A868:  57                           push     di
00A869:  1E                           push     ds
00A86A:  65 C7 06 A0 0A 01 00         mov      word ptr gs:[0xaa0], 1
00A871:  83 C6 06                     add      si, 6
00A874:  33 ED                        xor      bp, bp
00A876:  EB 28                        jmp      0xa8a0
; -- non-contiguous block: next 0x00a8a0 --
00A8A0:  D1 ED                        shr      bp, 1
00A8A2:  74 05                        je       0xa8a9
00A8A4:  73 0B                        jae      0xa8b1
00A8A6:  A4                           movsb    byte ptr es:[di], byte ptr [si]
00A8A7:  EB F7                        jmp      0xa8a0
00A8A9:  AD                           lodsw    ax, word ptr [si]
00A8AA:  8B E8                        mov      bp, ax
00A8AC:  F9                           stc     
00A8AD:  D1 DD                        rcr      bp, 1
00A8AF:  72 F5                        jb       0xa8a6
00A8B1:  33 C9                        xor      cx, cx
00A8B3:  D1 ED                        shr      bp, 1
00A8B5:  75 06                        jne      0xa8bd
00A8B7:  AD                           lodsw    ax, word ptr [si]
00A8B8:  8B E8                        mov      bp, ax
00A8BA:  F9                           stc     
00A8BB:  D1 DD                        rcr      bp, 1
00A8BD:  72 2E                        jb       0xa8ed
00A8BF:  D1 ED                        shr      bp, 1
00A8C1:  75 06                        jne      0xa8c9
00A8C3:  AD                           lodsw    ax, word ptr [si]
00A8C4:  8B E8                        mov      bp, ax
00A8C6:  F9                           stc     
00A8C7:  D1 DD                        rcr      bp, 1
00A8C9:  D1 D1                        rcl      cx, 1
00A8CB:  D1 ED                        shr      bp, 1
00A8CD:  75 06                        jne      0xa8d5
00A8CF:  AD                           lodsw    ax, word ptr [si]
00A8D0:  8B E8                        mov      bp, ax
00A8D2:  F9                           stc     
00A8D3:  D1 DD                        rcr      bp, 1
00A8D5:  D1 D1                        rcl      cx, 1
00A8D7:  AC                           lodsb    al, byte ptr [si]
00A8D8:  B4 FF                        mov      ah, 0xff
00A8DA:  03 C7                        add      ax, di
00A8DC:  96                           xchg     si, ax
00A8DD:  8C DB                        mov      bx, ds
00A8DF:  8C C2                        mov      dx, es
00A8E1:  8E DA                        mov      ds, dx
00A8E3:  41                           inc      cx
00A8E4:  41                           inc      cx
00A8E5:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00A8E7:  8E DB                        mov      ds, bx
00A8E9:  8B F0                        mov      si, ax
00A8EB:  EB B3                        jmp      0xa8a0
00A8ED:  AD                           lodsw    ax, word ptr [si]
00A8EE:  8A C8                        mov      cl, al
00A8F0:  D1 E8                        shr      ax, 1
00A8F2:  D1 E8                        shr      ax, 1
00A8F4:  D1 E8                        shr      ax, 1
00A8F6:  80 CC E0                     or       ah, 0xe0
00A8F9:  80 E1 07                     and      cl, 7
00A8FC:  75 DC                        jne      0xa8da
00A8FE:  8B D8                        mov      bx, ax
00A900:  AC                           lodsb    al, byte ptr [si]
00A901:  8A C8                        mov      cl, al
00A903:  8B C3                        mov      ax, bx
00A905:  0A C9                        or       cl, cl
00A907:  75 D1                        jne      0xa8da
00A909:  F9                           stc     
00A90A:  8B CF                        mov      cx, di
00A90C:  1F                           pop      ds
00A90D:  5F                           pop      di
00A90E:  83 C4 02                     add      sp, 2
00A911:  2B CF                        sub      cx, di
00A913:  C3                           ret     
