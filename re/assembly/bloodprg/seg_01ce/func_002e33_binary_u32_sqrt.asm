; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002e33
; seg_off: 01ce:0b53
; group: seg_01ce
; provenance: relocation_proven_far_transfer_target
; label: binary_u32_sqrt
; label_comment: integer square-root helper for DX:AX, used by ship 3D position distance
; incoming: call@0x00619a->01ce:0b53
; byte_count: 64
; boundary: cfg_blocks_10_terminals_3
; terminal: jmp 0x2e5c:2, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_002e33_binary_u32_sqrt.cpp
; routine_bytes_sha256: 053e6585212671dcd885fa828776a23bd65c13b9905c43cad9e20e9b6318b188

002E33:  53                           push     bx
002E34:  51                           push     cx
002E35:  52                           push     dx
002E36:  55                           push     bp
002E37:  8B C8                        mov      cx, ax
002E39:  8B EA                        mov      bp, dx
002E3B:  0B D2                        or       dx, dx
002E3D:  74 10                        je       0x2e4f
002E3F:  BB FF 0F                     mov      bx, 0xfff
002E42:  0A F6                        or       dh, dh
002E44:  74 16                        je       0x2e5c
002E46:  B7 FF                        mov      bh, 0xff
002E48:  83 FA FE                     cmp      dx, -2
002E4B:  73 21                        jae      0x2e6e
002E4D:  EB 0D                        jmp      0x2e5c
002E4F:  0B C0                        or       ax, ax
002E51:  74 1B                        je       0x2e6e
002E53:  BB 0F 00                     mov      bx, 0xf
002E56:  0A E4                        or       ah, ah
002E58:  74 02                        je       0x2e5c
002E5A:  B3 FF                        mov      bl, 0xff
002E5C:  F7 F3                        div      bx
002E5E:  03 C3                        add      ax, bx
002E60:  D1 D8                        rcr      ax, 1
002E62:  3B C3                        cmp      ax, bx
002E64:  73 08                        jae      0x2e6e
002E66:  8B D8                        mov      bx, ax
002E68:  8B C1                        mov      ax, cx
002E6A:  8B D5                        mov      dx, bp
002E6C:  EB EE                        jmp      0x2e5c
002E6E:  5D                           pop      bp
002E6F:  5A                           pop      dx
002E70:  59                           pop      cx
002E71:  5B                           pop      bx
002E72:  CB                           retf    
