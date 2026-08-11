; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001a93
; seg_off: 008b:0be3
; group: seg_008b
; provenance: recursive_graph
; label: main_loop_hud_refresh
; label_comment: main-loop per-frame HUD refresh (gated on [0xadf]&1): program VGA sequencer out 0x3c4,ax=0x0f02 (map-mask = all 4 planes), les di,[0x521d] (screen buffer far ptr), clear a 20-byte x 14-row region at row-stride 0x50 (80 bytes/row), then lcall 0x299:0x498 (graphics draw) + lcall 0:0x5d7. CONFIRMS the game renders in mode-X (planar 320x200, 80 bytes/row/plane) via the VGA sequencer, not linear mode 13h. The engine's linear framebuffer produces identical visual output (mode-X is a perf/paging technique, not a visual difference)
; byte_count: 64
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_008b/func_001a93_main_loop_hud_refresh.cpp
; routine_bytes_sha256: 8c191f412037f0fb6d4877e5fbbccb0bd32232216a8ce9e3e7a8855ae632cf67

001A93:  56                           push     si
001A94:  06                           push     es
001A95:  F6 06 DF 0A 01               test     byte ptr [0xadf], 1
001A9A:  74 34                        je       0x1ad0
001A9C:  BA C4 03                     mov      dx, 0x3c4
001A9F:  B8 02 0F                     mov      ax, 0xf02
001AA2:  EF                           out      dx, ax
001AA3:  C4 3E 1D 52                  les      di, ptr [0x521d]
001AA7:  81 C7 2E 1D                  add      di, 0x1d2e
001AAB:  B3 0E                        mov      bl, 0xe
001AAD:  32 C0                        xor      al, al
001AAF:  B9 14 00                     mov      cx, 0x14
001AB2:  F3 AA                        rep stosb byte ptr es:[di], al
001AB4:  83 C7 3C                     add      di, 0x3c
001AB7:  FE CB                        dec      bl
001AB9:  75 F4                        jne      0x1aaf
001ABB:  BE 66 01                     mov      si, 0x166
001ABE:  BB 87 00                     mov      bx, 0x87
001AC1:  BA 60 00                     mov      dx, 0x60
001AC4:  B0 E8                        mov      al, 0xe8
001AC6:  9A 98 04 99 02               lcall    0x299, 0x498
001ACB:  9A D7 05 00 00               lcall    0, 0x5d7
001AD0:  07                           pop      es
001AD1:  5E                           pop      si
001AD2:  C3                           ret     
