; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003e5b
; seg_off: 0299:0ecb
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: fullscreen_copy_to_backbuffer
; label_comment: full-screen copy into the LINEAR back-buffer: les di,gs:[0x5229]; cx=0x3e80 (16000 dwords = 64000 bytes = whole frame); rep movsd from si. Copies a whole 320x200 frame into the back buffer (vs full_screen_blit 0x3e46 which targets the display)
; incoming: call@0x00b140->0299:0ecb
; byte_count: 21
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 362c8ff17c1838d14a1315e2b6c262674d128bd857001f944b9d9f789618ee05

003E5B:  51                           push     cx
003E5C:  06                           push     es
003E5D:  57                           push     di
003E5E:  56                           push     si
003E5F:  FC                           cld     
003E60:  65 C4 3E 29 52               les      di, ptr gs:[0x5229]
003E65:  B9 80 3E                     mov      cx, 0x3e80
003E68:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
003E6B:  5E                           pop      si
003E6C:  5F                           pop      di
003E6D:  07                           pop      es
003E6E:  59                           pop      cx
003E6F:  CB                           retf    
