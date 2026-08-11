; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003b45
; seg_off: 0299:0bb5
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: composite_draw_a
; label_comment: composite shape draw: call 0x32ac + call gfx_clipped_draw 0x3321 (two plot primitives with xchg bp,dx between). Draws a filled/compound shape from the clipped primitives
; incoming: call@0x000e96->0299:0bb5
; incoming: call@0x0014f9->0299:0bb5
; incoming: call@0x007a62->0299:0bb5
; incoming: call@0x007ca8->0299:0bb5
; byte_count: 32
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: 0x0032ac, 0x003321
; indirect_calls: 0
; routine_bytes_sha256: 244b88c80dc7d12184b6a54d14e89ec241265ffff99a902cdc64a4ba07c8c539

003B45:  51                           push     cx
003B46:  0E                           push     cs
003B47:  E8 62 F7                     call     0x32ac
003B4A:  87 D5                        xchg     bp, dx
003B4C:  0E                           push     cs
003B4D:  E8 D1 F7                     call     0x3321
003B50:  03 DD                        add      bx, bp
003B52:  4B                           dec      bx
003B53:  0E                           push     cs
003B54:  E8 CA F7                     call     0x3321
003B57:  2B DD                        sub      bx, bp
003B59:  43                           inc      bx
003B5A:  87 D5                        xchg     bp, dx
003B5C:  03 CD                        add      cx, bp
003B5E:  49                           dec      cx
003B5F:  0E                           push     cs
003B60:  E8 49 F7                     call     0x32ac
003B63:  59                           pop      cx
003B64:  CB                           retf    
