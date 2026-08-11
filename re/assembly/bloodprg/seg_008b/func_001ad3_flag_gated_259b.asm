; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001ad3
; seg_off: 008b:0c23
; group: seg_008b
; provenance: recursive_graph
; label: flag_gated_259b
; label_comment: flag-gated routine: test byte [0x259b],1; if clear skip to 0x1b4a, else si=0x259d. Branches on the 0x259b enable bit and points at the 0x259d parameter block
; byte_count: 120
; boundary: cfg_blocks_13_terminals_1
; terminal: ret:1
; direct_callees: 0x001e5d
; indirect_calls: 2
; routine_bytes_sha256: 245ab7c35e055cac78510a7d33358c250623b597c8a326ad4649308f16d0f927

001AD3:  F6 06 9B 25 01               test     byte ptr [0x259b], 1
001AD8:  74 70                        je       0x1b4a
001ADA:  BE 9D 25                     mov      si, 0x259d
001ADD:  80 0E 93 27 04               or       byte ptr [0x2793], 4
001AE2:  F6 06 9C 25 01               test     byte ptr [0x259c], 1
001AE7:  74 1D                        je       0x1b06
001AE9:  C6 06 E6 27 01               mov      byte ptr [0x27e6], 1
001AEE:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
001AF3:  C6 06 E6 27 00               mov      byte ptr [0x27e6], 0
001AF8:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
001AFD:  C6 06 DA 0A 06               mov      byte ptr [0xada], 6
001B02:  FE 06 9C 25                  inc      byte ptr [0x259c]
001B06:  F6 06 9C 25 02               test     byte ptr [0x259c], 2
001B0B:  74 13                        je       0x1b20
001B0D:  56                           push     si
001B0E:  BE AB 2A                     mov      si, 0x2aab
001B11:  BF CF 25                     mov      di, 0x25cf
001B14:  0E                           push     cs
001B15:  E8 45 03                     call     0x1e5d
001B18:  5E                           pop      si
001B19:  73 2F                        jae      0x1b4a
001B1B:  C6 06 9C 25 00               mov      byte ptr [0x259c], 0
001B20:  9A 48 0C 1E 07               lcall    0x71e, 0xc48
001B25:  0B C0                        or       ax, ax
001B27:  78 21                        js       0x1b4a
001B29:  03 C0                        add      ax, ax
001B2B:  03 F0                        add      si, ax
001B2D:  83 3C FF                     cmp      word ptr [si], -1
001B30:  74 0E                        je       0x1b40
001B32:  83 F8 08                     cmp      ax, 8
001B35:  75 03                        jne      0x1b3a
001B37:  83 C0 04                     add      ax, 4
001B3A:  D1 E8                        shr      ax, 1
001B3C:  40                           inc      ax
001B3D:  A3 CA 0A                     mov      word ptr [0xaca], ax
001B40:  80 26 93 27 FB               and      byte ptr [0x2793], 0xfb
001B45:  C6 06 9B 25 00               mov      byte ptr [0x259b], 0
001B4A:  C3                           ret     
