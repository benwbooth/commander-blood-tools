; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0082c3
; seg_off: 071e:0ae3
; group: seg_071e
; provenance: recursive_graph
; label: ui_region_table_scan
; label_comment: the console's clickable-REGION scanner (returns a region id; 0x1F = the region selecting pose 0xC) — the hit-region table system behind the pose selectors and click dispatch. Selector contexts: 9 @0x7911 = region-enter edge ([0x27EA]); 0xC @0x7962 = region 0x1F hover; 0xE @0x7BEA = post-0x971-call state. Remaining sites sweep mechanically against this scanner's region ids || MERGED 2026-07-25 (audit-fixes #130), also recorded as: UI clickable-region table scan: bp=0x65f2; iterate 8-byte records (bl=[bp]; bp+=8); ax=0x1f (count). Walks the table of clickable UI regions/buttons at 0x65f2
; byte_count: 37
; boundary: cfg_blocks_6_terminals_1
; terminal: retf:1
; direct_callees: 0x008295
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_071e/func_0082c3_ui_region_table_scan.cpp
; routine_bytes_sha256: ed06b04960c68f21312006c2f4dfbf201f6e6200e202d110e8cad963d288bbff

0082C3:  55                           push     bp
0082C4:  53                           push     bx
0082C5:  B8 1F 00                     mov      ax, 0x1f
0082C8:  BD F2 65                     mov      bp, 0x65f2
0082CB:  8A 5E 00                     mov      bl, byte ptr [bp]
0082CE:  83 C5 08                     add      bp, 8
0082D1:  F6 C3 01                     test     bl, 1
0082D4:  74 06                        je       0x82dc
0082D6:  0E                           push     cs
0082D7:  E8 BB FF                     call     0x8295
0082DA:  72 09                        jb       0x82e5
0082DC:  83 ED 28                     sub      bp, 0x28
0082DF:  48                           dec      ax
0082E0:  79 E6                        jns      0x82c8
0082E2:  B8 FF FF                     mov      ax, 0xffff
0082E5:  5B                           pop      bx
0082E6:  5D                           pop      bp
0082E7:  CB                           retf    
