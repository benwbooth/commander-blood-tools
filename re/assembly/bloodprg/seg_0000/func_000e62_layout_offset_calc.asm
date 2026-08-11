; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000e62
; seg_off: 0086:0002
; group: seg_0000
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: layout_offset_calc
; label_comment: offset/layout helper used by set_vga_segment; computes centered screen positions and draws two helper primitives
; incoming: call@0x000da2->0x000e62
; incoming: call@0x000dcb->0x000e62
; incoming: call@0x000dfb->0x000e62
; byte_count: 71
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 2
; routine_bytes_sha256: 06775dcb53246fa18107ed636ffeff0a99507c238bca6cebbf4f75db05f27007

000E62:  51                           push     cx
000E63:  52                           push     dx
000E64:  55                           push     bp
000E65:  C1 E0 02                     shl      ax, 2
000E68:  83 C0 04                     add      ax, 4
000E6B:  8B D3                        mov      dx, bx
000E6D:  C1 E3 02                     shl      bx, 2
000E70:  03 DA                        add      bx, dx
000E72:  03 DA                        add      bx, dx
000E74:  83 C3 04                     add      bx, 4
000E77:  BA 40 01                     mov      dx, 0x140
000E7A:  2B D0                        sub      dx, ax
000E7C:  D1 EA                        shr      dx, 1
000E7E:  92                           xchg     dx, ax
000E7F:  BD C8 00                     mov      bp, 0xc8
000E82:  2B EB                        sub      bp, bx
000E84:  D1 ED                        shr      bp, 1
000E86:  87 DD                        xchg     bp, bx
000E88:  8B CB                        mov      cx, bx
000E8A:  8B D8                        mov      bx, ax
000E8C:  33 C0                        xor      ax, ax
000E8E:  9A DC 0C 99 02               lcall    0x299, 0xcdc
000E93:  B8 0F 00                     mov      ax, 0xf
000E96:  9A B5 0B 99 02               lcall    0x299, 0xbb5
000E9B:  83 C3 02                     add      bx, 2
000E9E:  8B C3                        mov      ax, bx
000EA0:  83 C1 02                     add      cx, 2
000EA3:  8B D9                        mov      bx, cx
000EA5:  5D                           pop      bp
000EA6:  5A                           pop      dx
000EA7:  59                           pop      cx
000EA8:  CB                           retf
