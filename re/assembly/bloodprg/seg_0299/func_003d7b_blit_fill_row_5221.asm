; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003d7b
; seg_off: 0299:0deb
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: blit_fill_row_5221
; label_comment: display-buffer band fill parallel to 0x003DBF: uses the segment from gs:[0x5221], wrapped row bounds at gs:0x5239/0x523b, AL replicated over EAX, and REP STOSD
; incoming: call@0x0016b7->0299:0deb
; incoming: call@0x001ec3->0299:0deb
; incoming: call@0x001f39->0299:0deb
; incoming: call@0x008b97->0299:0deb
; incoming: call@0x00955e->0299:0deb
; incoming: call@0x0095d4->0299:0deb
; incoming: call@0x009ed5->0299:0deb
; incoming: call@0x00b054->0299:0deb
; incoming: call@0x00b4fb->0299:0deb
; incoming: call@0x00b624->0299:0deb
; byte_count: 68
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 963263cfa7d2c9aade7cbf3e751fa2f8b31d0c5f0799fd2064243714ef262789

003D7B:  66 50                        push     eax
003D7D:  53                           push     bx
003D7E:  51                           push     cx
003D7F:  06                           push     es
003D80:  57                           push     di
003D81:  FC                           cld     
003D82:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
003D87:  65 8B 0E 39 52               mov      cx, word ptr gs:[0x5239]
003D8C:  8B D9                        mov      bx, cx
003D8E:  86 CD                        xchg     ch, cl
003D90:  8B F9                        mov      di, cx
003D92:  C1 E3 06                     shl      bx, 6
003D95:  03 FB                        add      di, bx
003D97:  65 8B 0E 3B 52               mov      cx, word ptr gs:[0x523b]
003D9C:  65 2B 0E 39 52               sub      cx, word ptr gs:[0x5239]
003DA1:  8B D9                        mov      bx, cx
003DA3:  C1 E1 06                     shl      cx, 6
003DA6:  C1 E3 04                     shl      bx, 4
003DA9:  03 CB                        add      cx, bx
003DAB:  8A E0                        mov      ah, al
003DAD:  8B D8                        mov      bx, ax
003DAF:  66 C1 E0 10                  shl      eax, 0x10
003DB3:  8B C3                        mov      ax, bx
003DB5:  F3 66 AB                     rep stosd dword ptr es:[di], eax
003DB8:  5F                           pop      di
003DB9:  07                           pop      es
003DBA:  59                           pop      cx
003DBB:  5B                           pop      bx
003DBC:  66 58                        pop      eax
003DBE:  CB                           retf    
