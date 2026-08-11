; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008269
; seg_off: 071e:0a89
; group: seg_071e
; provenance: recursive_graph
; label: mouse_hit_test
; label_comment: box hit test: si -> {x,y,w,h} 8-byte rect; inside = [0xa2a]>=x && [0xa2a]-w<=x && [0xa2c]>=y && [0xa2c]-h<=y; on hit or byte [bp],8. PROVES the station/orb rect field order {x,y,w,h} (tbbig chunk header + station table +0xc) || MERGED 2026-07-25 (audit-fixes #130), also recorded as: UI mouse hit-test (family 0x8269/0x8295): gated on mouse-button flag [0xa3e]; ax=mouse-x [0xa2a]; cmp vs a region bound [si]/[bp]. Detects clicks in a screen region - the UI/HUD click hit-testing
; byte_count: 44
; boundary: cfg_blocks_7_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 4d58fbabbdbb394b7434f1a085d2b779a56bcb414939a27c43a5652f58895962

008269:  50                           push     ax
00826A:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
00826F:  74 22                        je       0x8293
008271:  A1 2A 0A                     mov      ax, word ptr [0xa2a]
008274:  3B 04                        cmp      ax, word ptr [si]
008276:  7C 1B                        jl       0x8293
008278:  2B 44 04                     sub      ax, word ptr [si + 4]
00827B:  3B 04                        cmp      ax, word ptr [si]
00827D:  7F 14                        jg       0x8293
00827F:  A1 2C 0A                     mov      ax, word ptr [0xa2c]
008282:  3B 44 02                     cmp      ax, word ptr [si + 2]
008285:  7C 0C                        jl       0x8293
008287:  2B 44 06                     sub      ax, word ptr [si + 6]
00828A:  3B 44 02                     cmp      ax, word ptr [si + 2]
00828D:  7F 04                        jg       0x8293
00828F:  80 4E 00 08                  or       byte ptr [bp], 8
008293:  58                           pop      ax
008294:  C3                           ret     
