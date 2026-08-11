; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002fa6
; seg_off: 0299:0016
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vga_dac_clear
; label_comment: NAME CORRECTED (audit-fixes #412; was gfx_display_draw_family, which described a family this routine is not in). 0x2FA6 is the VGA DAC BLANK: push ax/cx/dx; mov dx,0x3c8; xor al,al; out dx,al (reset the PEL write index); inc dl (-> 0x3c9); mov cx,0x300; then `out dx,al` / `loop 0x2FB4` writing 768 ZERO bytes = all 256 palette entries blanked; pop dx/cx/ax. It contains NO lds/les and never touches gs:[0x5221]. The draw family IS real but starts at 0x2FBB -- 0x2FBB and 0x3000 both open `lds si, ptr gs:[0x5221]` and already carry their own labels blit_page_5221_a/_b. PORT: src/recomp/io_lift.rs func_2fa6, which had the right name all along
; incoming: call@0x000c5a->0299:0016
; incoming: call@0x001f34->0299:0016
; byte_count: 21
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_002fa6_vga_dac_clear.cpp
; routine_bytes_sha256: ce29048be80390a42628857ed4689ef168bf4364ca84cda03b34bddda6a697ab

002FA6:  50                           push     ax
002FA7:  51                           push     cx
002FA8:  52                           push     dx
002FA9:  BA C8 03                     mov      dx, 0x3c8
002FAC:  32 C0                        xor      al, al
002FAE:  EE                           out      dx, al
002FAF:  FE C2                        inc      dl
002FB1:  B9 00 03                     mov      cx, 0x300
002FB4:  EE                           out      dx, al
002FB5:  E2 FD                        loop     0x2fb4
002FB7:  5A                           pop      dx
002FB8:  59                           pop      cx
002FB9:  58                           pop      ax
002FBA:  CB                           retf    
