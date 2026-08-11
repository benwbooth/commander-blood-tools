; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001e5d
; seg_off: 008b:0fad
; group: seg_008b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: ship_3d_interpolation_gate
; label_comment: four-word interpolation gate; carry clear while drawing, carry set when DS:0x0ADB reaches DS:0x0ADA
; incoming: call@0x008778->008b:0fad
; incoming: call@0x008808->008b:0fad
; incoming: call@0x0088aa->008b:0fad
; incoming: call@0x0089e3->008b:0fad
; incoming: call@0x008a18->008b:0fad
; incoming: call@0x009117->008b:0fad
; incoming: call@0x009209->008b:0fad
; incoming: call@0x00b30b->008b:0fad
; incoming: call@0x00b4bc->008b:0fad
; byte_count: 100
; boundary: cfg_blocks_4_terminals_2
; terminal: jmp 0x1eba:1, retf:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_008b/func_001e5d_ship_3d_interpolation_gate.cpp
; routine_bytes_sha256: d10a38f7b513426a28ffb9f8cb926da1132159fdeeea5a161db3aa5705f7a8b1

001E5D:  50                           push     ax
001E5E:  53                           push     bx
001E5F:  51                           push     cx
001E60:  52                           push     dx
001E61:  55                           push     bp
001E62:  56                           push     si
001E63:  8A 1E DA 0A                  mov      bl, byte ptr [0xada]
001E67:  3A 1E DB 0A                  cmp      bl, byte ptr [0xadb]
001E6B:  74 4C                        je       0x1eb9
001E6D:  FE 06 DB 0A                  inc      byte ptr [0xadb]
001E71:  AD                           lodsw    ax, word ptr [si]
001E72:  2B 05                        sub      ax, word ptr [di]
001E74:  F6 FB                        idiv     bl
001E76:  F6 2E DB 0A                  imul     byte ptr [0xadb]
001E7A:  8B 15                        mov      dx, word ptr [di]
001E7C:  03 D0                        add      dx, ax
001E7E:  52                           push     dx
001E7F:  AD                           lodsw    ax, word ptr [si]
001E80:  2B 45 02                     sub      ax, word ptr [di + 2]
001E83:  F6 FB                        idiv     bl
001E85:  F6 2E DB 0A                  imul     byte ptr [0xadb]
001E89:  8B 4D 02                     mov      cx, word ptr [di + 2]
001E8C:  03 C8                        add      cx, ax
001E8E:  AD                           lodsw    ax, word ptr [si]
001E8F:  2B 45 04                     sub      ax, word ptr [di + 4]
001E92:  F6 FB                        idiv     bl
001E94:  F6 2E DB 0A                  imul     byte ptr [0xadb]
001E98:  8B 55 04                     mov      dx, word ptr [di + 4]
001E9B:  03 D0                        add      dx, ax
001E9D:  AD                           lodsw    ax, word ptr [si]
001E9E:  2B 45 06                     sub      ax, word ptr [di + 6]
001EA1:  F6 FB                        idiv     bl
001EA3:  F6 2E DB 0A                  imul     byte ptr [0xadb]
001EA7:  8B 6D 06                     mov      bp, word ptr [di + 6]
001EAA:  03 E8                        add      bp, ax
001EAC:  5B                           pop      bx
001EAD:  8B 36 C8 0A                  mov      si, word ptr [0xac8]
001EB1:  9A 0E 04 99 02               lcall    0x299, 0x40e
001EB6:  F8                           clc     
001EB7:  EB 01                        jmp      0x1eba
001EB9:  F9                           stc     
001EBA:  5E                           pop      si
001EBB:  5D                           pop      bp
001EBC:  5A                           pop      dx
001EBD:  59                           pop      cx
001EBE:  5B                           pop      bx
001EBF:  58                           pop      ax
001EC0:  CB                           retf    
