; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002f90
; seg_off: 0299:0000
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vga_palette_write
; label_comment: VGA palette (DAC) write: dx=0x3c8; out dx,al (PEL write index); inc dl (0x3c9 = PEL data); cx=0x300 (768 = 256 colors * 3). Uploads the full 256-entry VGA palette to the DAC
; incoming: call@0x0016b0->0299:0000
; incoming: call@0x00179a->0299:0000
; byte_count: 22
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d7a784d4a48ec96200f782d433b961f34131658870c5761c8d112fe7b2bfcb08

002F90:  50                           push     ax
002F91:  51                           push     cx
002F92:  52                           push     dx
002F93:  56                           push     si
002F94:  BA C8 03                     mov      dx, 0x3c8
002F97:  32 C0                        xor      al, al
002F99:  EE                           out      dx, al
002F9A:  FE C2                        inc      dl
002F9C:  B9 00 03                     mov      cx, 0x300
002F9F:  F3 6E                        rep outsb dx, byte ptr [si]
002FA1:  5E                           pop      si
002FA2:  5A                           pop      dx
002FA3:  59                           pop      cx
002FA4:  58                           pop      ax
002FA5:  CB                           retf    
