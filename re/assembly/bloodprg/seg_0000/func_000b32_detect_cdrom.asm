; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000b32
; seg_off: 0000:0532
; group: seg_0000
; provenance: recursive_graph
; label: detect_cdrom
; label_comment: startup: ax=0x1500; int 2Fh (MSCDEX/CD-ROM installation check); bx=drive count. Detects the CD-ROM (the game ships on CD)
; byte_count: 16
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 8cec90cd480b3a3987d65e72cdf0065bc3cf5dfe4c785b87f5d69906562437bb

000B32:  B8 00 15                     mov      ax, 0x1500
000B35:  33 DB                        xor      bx, bx
000B37:  CD 2F                        int      0x2f
000B39:  0B DB                        or       bx, bx
000B3B:  65 0F 95 06 E6 0A            setne    byte ptr gs:[0xae6]
000B41:  C3                           ret     
