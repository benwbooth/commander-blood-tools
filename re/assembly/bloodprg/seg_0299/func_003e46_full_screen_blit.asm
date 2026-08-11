; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003e46
; seg_off: 0299:0eb6
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: full_screen_blit
; label_comment: full-screen copy: les di,gs:[0x5221] (display); cx=0x3e80 (16000); rep movsd (16000 dwords = 64000 bytes = entire 320x200 frame) from si into the display buffer. The full-frame present/copy (whole-screen page copy)
; incoming: call@0x0018e4->0299:0eb6
; incoming: call@0x00a492->0299:0eb6
; incoming: call@0x00a4aa->0299:0eb6
; incoming: call@0x00b4a4->0299:0eb6
; byte_count: 21
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 6911992622456f7740750a31025560aa2df326de8988938338d383bbace87dfd

003E46:  51                           push     cx
003E47:  06                           push     es
003E48:  57                           push     di
003E49:  56                           push     si
003E4A:  FC                           cld     
003E4B:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
003E50:  B9 80 3E                     mov      cx, 0x3e80
003E53:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
003E56:  5E                           pop      si
003E57:  5F                           pop      di
003E58:  07                           pop      es
003E59:  59                           pop      cx
003E5A:  CB                           retf    
