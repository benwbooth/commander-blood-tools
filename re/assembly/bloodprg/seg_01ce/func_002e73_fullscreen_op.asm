; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002e73
; seg_off: 01ce:0b93
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: far_memmove
; label_comment: overlap-safe far memmove of EAX bytes from DS:SI to ES:DI. Converts both far pointers to linear addresses, selects backward copy when source overlaps destination, and normalizes the pointers in chunks of at most 0xfa00 (64000) bytes; preserves all registers and returns with direction clear.
; incoming: call@0x005302->01ce:0b93
; incoming: call@0x00a6aa->01ce:0b93
; incoming: call@0x00b95f->01ce:0b93
; byte_count: 237
; boundary: cfg_blocks_16_terminals_6
; terminal: jmp 0x2ec0:1, jmp 0x2f0b:2, jmp 0x2f50:2, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d7644d9198fc977af470a84a24d3e6898c5b0566c644ed196ea0ce845e158342

002E73:  66 50                        push     eax
002E75:  66 53                        push     ebx
002E77:  66 51                        push     ecx
002E79:  66 52                        push     edx
002E7B:  1E                           push     ds
002E7C:  56                           push     si
002E7D:  06                           push     es
002E7E:  57                           push     di
002E7F:  66 55                        push     ebp
002E81:  FC                           cld     
002E82:  66 BD 00 FA 00 00            mov      ebp, 0xfa00
002E88:  66 33 DB                     xor      ebx, ebx
002E8B:  66 8B D3                     mov      edx, ebx
002E8E:  66 8B CB                     mov      ecx, ebx
002E91:  8C DB                        mov      bx, ds
002E93:  8B CE                        mov      cx, si
002E95:  66 C1 E3 04                  shl      ebx, 4
002E99:  66 03 D9                     add      ebx, ecx
002E9C:  8C C2                        mov      dx, es
002E9E:  8B CF                        mov      cx, di
002EA0:  66 C1 E2 04                  shl      edx, 4
002EA4:  66 03 D1                     add      edx, ecx
002EA7:  66 3B DA                     cmp      ebx, edx
002EAA:  7F 14                        jg       0x2ec0
002EAC:  66 8B CB                     mov      ecx, ebx
002EAF:  66 03 C8                     add      ecx, eax
002EB2:  66 3B CA                     cmp      ecx, edx
002EB5:  7C 09                        jl       0x2ec0
002EB7:  66 8B D9                     mov      ebx, ecx
002EBA:  66 03 D0                     add      edx, eax
002EBD:  FD                           std     
002EBE:  EB 4B                        jmp      0x2f0b
002EC0:  66 8B CB                     mov      ecx, ebx
002EC3:  8B F3                        mov      si, bx
002EC5:  83 E6 0F                     and      si, 0xf
002EC8:  66 C1 E9 04                  shr      ecx, 4
002ECC:  8E D9                        mov      ds, cx
002ECE:  66 8B CA                     mov      ecx, edx
002ED1:  8B FA                        mov      di, dx
002ED3:  83 E7 0F                     and      di, 0xf
002ED6:  66 C1 E9 04                  shr      ecx, 4
002EDA:  8E C1                        mov      es, cx
002EDC:  66 8B CD                     mov      ecx, ebp
002EDF:  66 2B C1                     sub      eax, ecx
002EE2:  79 19                        jns      0x2efd
002EE4:  F7 D8                        neg      ax
002EE6:  2B C8                        sub      cx, ax
002EE8:  8B C1                        mov      ax, cx
002EEA:  C1 E9 02                     shr      cx, 2
002EED:  74 03                        je       0x2ef2
002EEF:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
002EF2:  83 E0 03                     and      ax, 3
002EF5:  74 59                        je       0x2f50
002EF7:  8B C8                        mov      cx, ax
002EF9:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
002EFB:  EB 53                        jmp      0x2f50
002EFD:  C1 E9 02                     shr      cx, 2
002F00:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
002F03:  66 03 DD                     add      ebx, ebp
002F06:  66 03 D5                     add      edx, ebp
002F09:  EB B5                        jmp      0x2ec0
002F0B:  8B F5                        mov      si, bp
002F0D:  8B FD                        mov      di, bp
002F0F:  66 2B DD                     sub      ebx, ebp
002F12:  66 2B D5                     sub      edx, ebp
002F15:  66 8B CB                     mov      ecx, ebx
002F18:  66 C1 E9 04                  shr      ecx, 4
002F1C:  8E D9                        mov      ds, cx
002F1E:  66 8B CA                     mov      ecx, edx
002F21:  66 C1 E9 04                  shr      ecx, 4
002F25:  8E C1                        mov      es, cx
002F27:  66 8B CD                     mov      ecx, ebp
002F2A:  66 2B C1                     sub      eax, ecx
002F2D:  79 19                        jns      0x2f48
002F2F:  F7 D8                        neg      ax
002F31:  2B C8                        sub      cx, ax
002F33:  8B C1                        mov      ax, cx
002F35:  C1 E9 02                     shr      cx, 2
002F38:  74 03                        je       0x2f3d
002F3A:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
002F3D:  83 E0 03                     and      ax, 3
002F40:  74 0E                        je       0x2f50
002F42:  8B C8                        mov      cx, ax
002F44:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
002F46:  EB 08                        jmp      0x2f50
002F48:  C1 E9 02                     shr      cx, 2
002F4B:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
002F4E:  EB BB                        jmp      0x2f0b
002F50:  FC                           cld     
002F51:  66 5D                        pop      ebp
002F53:  5F                           pop      di
002F54:  07                           pop      es
002F55:  5E                           pop      si
002F56:  1F                           pop      ds
002F57:  66 5A                        pop      edx
002F59:  66 59                        pop      ecx
002F5B:  66 5B                        pop      ebx
002F5D:  66 58                        pop      eax
002F5F:  CB                           retf    
