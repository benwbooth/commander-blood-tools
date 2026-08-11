; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002abb
; seg_off: 01ce:07db
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_file_load
; label_comment: SEG 0x1ce:0x7db: open the resource filename (via gs-relative path builder 0x2693) + read the file into the allocated resource segment; the loaded world/.ext data then lives at the resource segment (see resource_handle_resolve 0x5320)
; incoming: call@0x000f48->01ce:07db
; incoming: call@0x000f86->01ce:07db
; incoming: call@0x007e49->01ce:07db
; incoming: call@0x00b5cd->01ce:07db
; incoming: call@0x00b61d->01ce:07db
; byte_count: 176
; boundary: cfg_blocks_12_terminals_3
; terminal: jmp 0x2b60:1, jmp 0x2b63:1, retf:1
; direct_callees: 0x002693
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_002abb_resource_file_load.cpp
; routine_bytes_sha256: b5b1a9c724a4f88b38f924bb265aa4f04b4a443234d9ac22c3b983f5f0e6ce30

002ABB:  53                           push     bx
002ABC:  66 51                        push     ecx
002ABE:  52                           push     dx
002ABF:  06                           push     es
002AC0:  1E                           push     ds
002AC1:  56                           push     si
002AC2:  06                           push     es
002AC3:  8B D6                        mov      dx, si
002AC5:  0E                           push     cs
002AC6:  E8 CA FB                     call     0x2693
002AC9:  8B C3                        mov      ax, bx
002ACB:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
002AD1:  75 2B                        jne      0x2afe
002AD3:  B8 00 2F                     mov      ax, 0x2f00
002AD6:  CD 21                        int      0x21
002AD8:  8B F3                        mov      si, bx
002ADA:  83 C6 1A                     add      si, 0x1a
002ADD:  33 C9                        xor      cx, cx
002ADF:  B8 00 4E                     mov      ax, 0x4e00
002AE2:  CD 21                        int      0x21
002AE4:  66 26 8B 0C                  mov      ecx, dword ptr es:[si]
002AE8:  66 65 89 0E 8E 0A            mov      dword ptr gs:[0xa8e], ecx
002AEE:  66 65 89 0E 92 0A            mov      dword ptr gs:[0xa92], ecx
002AF4:  B8 00 3D                     mov      ax, 0x3d00
002AF7:  CD 21                        int      0x21
002AF9:  73 03                        jae      0x2afe
002AFB:  1F                           pop      ds
002AFC:  EB 62                        jmp      0x2b60
002AFE:  1F                           pop      ds
002AFF:  65 A3 84 0A                  mov      word ptr gs:[0xa84], ax
002B03:  8B D7                        mov      dx, di
002B05:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
002B0A:  66 B9 00 7D 00 00            mov      ecx, 0x7d00
002B10:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
002B15:  66 2B C1                     sub      eax, ecx
002B18:  79 02                        jns      0x2b1c
002B1A:  03 C8                        add      cx, ax
002B1C:  B8 00 3F                     mov      ax, 0x3f00
002B1F:  CD 21                        int      0x21
002B21:  65 29 06 92 0A               sub      word ptr gs:[0xa92], ax
002B26:  65 83 1E 94 0A 00            sbb      word ptr gs:[0xa94], 0
002B2C:  8B D8                        mov      bx, ax
002B2E:  C1 EB 04                     shr      bx, 4
002B31:  83 E0 0F                     and      ax, 0xf
002B34:  8C D9                        mov      cx, ds
002B36:  03 CB                        add      cx, bx
002B38:  8E D9                        mov      ds, cx
002B3A:  03 D0                        add      dx, ax
002B3C:  66 65 8B 0E 92 0A            mov      ecx, dword ptr gs:[0xa92]
002B42:  66 0B C9                     or       ecx, ecx
002B45:  75 BE                        jne      0x2b05
002B47:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
002B4D:  75 0A                        jne      0x2b59
002B4F:  B8 00 3E                     mov      ax, 0x3e00
002B52:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
002B57:  CD 21                        int      0x21
002B59:  66 65 A1 8E 0A               mov      eax, dword ptr gs:[0xa8e]
002B5E:  EB 03                        jmp      0x2b63
002B60:  66 33 C0                     xor      eax, eax
002B63:  5E                           pop      si
002B64:  1F                           pop      ds
002B65:  07                           pop      es
002B66:  5A                           pop      dx
002B67:  66 59                        pop      ecx
002B69:  5B                           pop      bx
002B6A:  CB                           retf    
