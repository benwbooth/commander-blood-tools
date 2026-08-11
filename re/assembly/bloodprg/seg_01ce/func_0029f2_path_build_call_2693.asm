; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0029f2
; seg_off: 01ce:0712
; group: seg_01ce
; provenance: relocation_proven_far_transfer_target
; label: path_build_call_2693
; label_comment: resource path build: dx=si; push cs; call 0x2693 (the path-string builder). Assembles the on-disk file path for a resource before FindFirst/open
; incoming: call@0x007728->01ce:0712
; byte_count: 201
; boundary: cfg_blocks_12_terminals_3
; terminal: jmp 0x2aae:1, jmp 0x2ab1:1, retf:1
; direct_callees: 0x002693
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_0029f2_path_build_call_2693.cpp
; routine_bytes_sha256: 5fb6e58818cf21a363df36ed0fb7601aadb7a0ba13e78244c3b8aa2b15d20cfa

0029F2:  66 50                        push     eax
0029F4:  53                           push     bx
0029F5:  51                           push     cx
0029F6:  52                           push     dx
0029F7:  1E                           push     ds
0029F8:  56                           push     si
0029F9:  06                           push     es
0029FA:  57                           push     di
0029FB:  8B D6                        mov      dx, si
0029FD:  0E                           push     cs
0029FE:  E8 92 FC                     call     0x2693
002A01:  8B C3                        mov      ax, bx
002A03:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
002A09:  75 2B                        jne      0x2a36
002A0B:  B8 00 2F                     mov      ax, 0x2f00
002A0E:  CD 21                        int      0x21
002A10:  8B F3                        mov      si, bx
002A12:  83 C6 1A                     add      si, 0x1a
002A15:  33 C9                        xor      cx, cx
002A17:  B8 00 4E                     mov      ax, 0x4e00
002A1A:  CD 21                        int      0x21
002A1C:  66 26 8B 04                  mov      eax, dword ptr es:[si]
002A20:  66 65 A3 8E 0A               mov      dword ptr gs:[0xa8e], eax
002A25:  66 65 A3 92 0A               mov      dword ptr gs:[0xa92], eax
002A2A:  B8 00 3D                     mov      ax, 0x3d00
002A2D:  CD 21                        int      0x21
002A2F:  73 05                        jae      0x2a36
002A31:  B8 01 00                     mov      ax, 1
002A34:  EB 78                        jmp      0x2aae
002A36:  65 A3 84 0A                  mov      word ptr gs:[0xa84], ax
002A3A:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
002A3F:  66 65 C7 06 4E 0A 00 00 00 00 mov      dword ptr gs:[0xa4e], 0
002A49:  66 33 C0                     xor      eax, eax
002A4C:  65 8B 1E 4E 0A               mov      bx, word ptr gs:[0xa4e]
002A51:  32 C0                        xor      al, al
002A53:  B9 02 00                     mov      cx, 2
002A56:  65 8B 16 58 0A               mov      dx, word ptr gs:[0xa58]
002A5B:  B4 44                        mov      ah, 0x44
002A5D:  CD 67                        int      0x67
002A5F:  43                           inc      bx
002A60:  FE C0                        inc      al
002A62:  E2 F2                        loop     0x2a56
002A64:  65 89 1E 4E 0A               mov      word ptr gs:[0xa4e], bx
002A69:  33 D2                        xor      dx, dx
002A6B:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
002A70:  B9 00 80                     mov      cx, 0x8000
002A73:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
002A78:  66 2B C1                     sub      eax, ecx
002A7B:  79 02                        jns      0x2a7f
002A7D:  03 C8                        add      cx, ax
002A7F:  B8 00 3F                     mov      ax, 0x3f00
002A82:  CD 21                        int      0x21
002A84:  66 0F B7 C0                  movzx    eax, ax
002A88:  66 65 29 06 92 0A            sub      dword ptr gs:[0xa92], eax
002A8E:  75 B9                        jne      0x2a49
002A90:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
002A96:  75 0A                        jne      0x2aa2
002A98:  B8 00 3E                     mov      ax, 0x3e00
002A9B:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
002AA0:  CD 21                        int      0x21
002AA2:  66 65 A1 8E 0A               mov      eax, dword ptr gs:[0xa8e]
002AA7:  66 65 A3 52 0A               mov      dword ptr gs:[0xa52], eax
002AAC:  EB 03                        jmp      0x2ab1
002AAE:  66 33 C0                     xor      eax, eax
002AB1:  5F                           pop      di
002AB2:  07                           pop      es
002AB3:  5E                           pop      si
002AB4:  1F                           pop      ds
002AB5:  5A                           pop      dx
002AB6:  59                           pop      cx
002AB7:  5B                           pop      bx
002AB8:  66 58                        pop      eax
002ABA:  CB                           retf    
