; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009b04
; seg_off: 071e:2324
; group: seg_071e
; provenance: recursive_graph
; label: ship_3d_plot_point
; label_comment: Point-cloud PLOT. Signed clip against DS:0x5235/0x5237 (x) and DS:0x5239/0x523B (y) with JL/JGE. Address = y*320 built as XCHG BH,BL (y*256) + SHL DI,6 (y*64), then + x. OCCLUSION: MOV AL,es:[di] / OR AL,AL / JNE -- a point is written ONLY where the pixel is still zero, so nearer points drawn first win. Shade = 0xEF - (depth >> 12) via SHR AX,0xC / NEG AL / ADD AL,0xEF. Ported exactly as ship3d.rs plot_ship_3d_projected_point + ship_3d_projected_point_shade/offset || ALSO RECORDED as `ship_3d_projected_point_plot`: clips projected point against DS:0x5235..0x523B and writes depth shade into ES framebuffer if empty || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; byte_count: 68
; boundary: cfg_blocks_7_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ac19f28f8de11959599f3709ac9a949cf4c83428d206d71a312b3cba58fd68a2

009B04:  50                           push     ax
009B05:  53                           push     bx
009B06:  57                           push     di
009B07:  8B 46 24                     mov      ax, word ptr [bp + 0x24]
009B0A:  3B 06 35 52                  cmp      ax, word ptr [0x5235]
009B0E:  7C 34                        jl       0x9b44
009B10:  3B 06 37 52                  cmp      ax, word ptr [0x5237]
009B14:  7D 2E                        jge      0x9b44
009B16:  8B 5E 26                     mov      bx, word ptr [bp + 0x26]
009B19:  3B 1E 39 52                  cmp      bx, word ptr [0x5239]
009B1D:  7C 25                        jl       0x9b44
009B1F:  3B 1E 3B 52                  cmp      bx, word ptr [0x523b]
009B23:  7D 1F                        jge      0x9b44
009B25:  8B FB                        mov      di, bx
009B27:  86 DF                        xchg     bh, bl
009B29:  C1 E7 06                     shl      di, 6
009B2C:  03 FB                        add      di, bx
009B2E:  03 F8                        add      di, ax
009B30:  26 8A 05                     mov      al, byte ptr es:[di]
009B33:  0A C0                        or       al, al
009B35:  75 0D                        jne      0x9b44
009B37:  8B 46 28                     mov      ax, word ptr [bp + 0x28]
009B3A:  C1 E8 0C                     shr      ax, 0xc
009B3D:  F6 D8                        neg      al
009B3F:  04 EF                        add      al, 0xef
009B41:  26 88 05                     mov      byte ptr es:[di], al
009B44:  5F                           pop      di
009B45:  5B                           pop      bx
009B46:  58                           pop      ax
009B47:  C3                           ret     
