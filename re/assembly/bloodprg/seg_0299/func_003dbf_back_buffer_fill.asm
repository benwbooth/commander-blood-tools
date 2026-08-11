; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003dbf
; seg_off: 0299:0e2f
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: back_buffer_fill
; label_comment: SEG 0x299:region: fill a horizontal band of the LINEAR back-buffer (gs:0x5229) with colour al. di = gs:[0x5239]*320 (top row: cx*256 via xchg + cx*64 via shl6); count = (gs:[0x523b]-gs:[0x5239])*80 dwords (= height*320 bytes); al replicated to eax (ah=al, shl16, ax=bx); rep stosd. Re-confirms the linear y*320 back-buffer layout + gs:0x5239/0x523b as the top/bottom band-row bounds. A screen/band clear before compositing
; incoming: call@0x00190d->0299:0e2f
; incoming: call@0x001eca->0299:0e2f
; incoming: call@0x001f40->0299:0e2f
; incoming: call@0x009dd5->0299:0e2f
; incoming: call@0x00b415->0299:0e2f
; byte_count: 68
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d096dbe66ed768141f59cac7ca0194e475163b977b3e157a27a9e6146de8a490

003DBF:  66 50                        push     eax
003DC1:  53                           push     bx
003DC2:  51                           push     cx
003DC3:  06                           push     es
003DC4:  57                           push     di
003DC5:  FC                           cld     
003DC6:  65 C4 3E 29 52               les      di, ptr gs:[0x5229]
003DCB:  65 8B 0E 39 52               mov      cx, word ptr gs:[0x5239]
003DD0:  8B D9                        mov      bx, cx
003DD2:  86 CD                        xchg     ch, cl
003DD4:  8B F9                        mov      di, cx
003DD6:  C1 E3 06                     shl      bx, 6
003DD9:  03 FB                        add      di, bx
003DDB:  65 8B 0E 3B 52               mov      cx, word ptr gs:[0x523b]
003DE0:  65 2B 0E 39 52               sub      cx, word ptr gs:[0x5239]
003DE5:  8B D9                        mov      bx, cx
003DE7:  C1 E1 06                     shl      cx, 6
003DEA:  C1 E3 04                     shl      bx, 4
003DED:  03 CB                        add      cx, bx
003DEF:  8A E0                        mov      ah, al
003DF1:  8B D8                        mov      bx, ax
003DF3:  66 C1 E0 10                  shl      eax, 0x10
003DF7:  8B C3                        mov      ax, bx
003DF9:  F3 66 AB                     rep stosd dword ptr es:[di], eax
003DFC:  5F                           pop      di
003DFD:  07                           pop      es
003DFE:  59                           pop      cx
003DFF:  5B                           pop      bx
003E00:  66 58                        pop      eax
003E02:  CB                           retf    
