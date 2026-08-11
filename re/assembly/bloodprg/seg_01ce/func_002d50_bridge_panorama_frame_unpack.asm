; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002d50
; seg_off: 01ce:0a70
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: bridge_panorama_frame_unpack
; label_comment: (= far 0x1CE:0x0A70) TB.BIG panorama-frame RLE unpacker, NOT a clear (old label wrong): les di,gs:[0x5229]; ebp=0xfa00 = decodes EXACTLY 64000 px (full 320x200) from ds:si. Signed control byte: <0 -> run of (-ctrl+1) copies of next byte, >=0 -> (ctrl+1) literals. gs:[0x5b57]&1 selects TRANSPARENT variant (value 0 skips: window starfield/prev frame shows through) vs OPAQUE. Ported+verified: src/tbbig.rs
; incoming: call@0x00988e->01ce:0a70
; byte_count: 110
; boundary: cfg_blocks_14_terminals_6
; terminal: jmp 0x2d68:3, jmp 0x2d9b:2, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 3fee20d60c4cfd5bebdf4d5bb6ab915b147a1483c2d353a4edbacff915a4f2b4

002D50:  65 C4 3E 29 52               les      di, ptr gs:[0x5229]
002D55:  66 33 C0                     xor      eax, eax
002D58:  8B C8                        mov      cx, ax
002D5A:  66 BD 00 FA 00 00            mov      ebp, 0xfa00
002D60:  65 F6 06 57 5B 01            test     byte ptr gs:[0x5b57], 1
002D66:  74 33                        je       0x2d9b
002D68:  0B ED                        or       bp, bp
002D6A:  74 51                        je       0x2dbd
002D6C:  AC                           lodsb    al, byte ptr [si]
002D6D:  0A C0                        or       al, al
002D6F:  79 16                        jns      0x2d87
002D71:  F6 D8                        neg      al
002D73:  FE C0                        inc      al
002D75:  8A C8                        mov      cl, al
002D77:  66 2B E8                     sub      ebp, eax
002D7A:  AC                           lodsb    al, byte ptr [si]
002D7B:  0A C0                        or       al, al
002D7D:  75 04                        jne      0x2d83
002D7F:  03 F9                        add      di, cx
002D81:  EB E5                        jmp      0x2d68
002D83:  F3 AA                        rep stosb byte ptr es:[di], al
002D85:  EB E1                        jmp      0x2d68
002D87:  FE C0                        inc      al
002D89:  8A C8                        mov      cl, al
002D8B:  66 2B E8                     sub      ebp, eax
002D8E:  AC                           lodsb    al, byte ptr [si]
002D8F:  0A C0                        or       al, al
002D91:  74 03                        je       0x2d96
002D93:  26 88 05                     mov      byte ptr es:[di], al
002D96:  47                           inc      di
002D97:  E2 F5                        loop     0x2d8e
002D99:  EB CD                        jmp      0x2d68
002D9B:  0B ED                        or       bp, bp
002D9D:  74 1E                        je       0x2dbd
002D9F:  AC                           lodsb    al, byte ptr [si]
002DA0:  0A C0                        or       al, al
002DA2:  79 0E                        jns      0x2db2
002DA4:  F6 D8                        neg      al
002DA6:  FE C0                        inc      al
002DA8:  8A C8                        mov      cl, al
002DAA:  66 2B E8                     sub      ebp, eax
002DAD:  AC                           lodsb    al, byte ptr [si]
002DAE:  F3 AA                        rep stosb byte ptr es:[di], al
002DB0:  EB E9                        jmp      0x2d9b
002DB2:  FE C0                        inc      al
002DB4:  8A C8                        mov      cl, al
002DB6:  66 2B E8                     sub      ebp, eax
002DB9:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
002DBB:  EB DE                        jmp      0x2d9b
002DBD:  CB                           retf    
