; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0028ca
; seg_off: 01ce:05ea
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_name_lookup
; label_comment: SEG 0x1ce:0x5ea: given DS:SI = FS:0x0c04 name-table entry, resolve/check the resource; returns ebp (0 = skip/already-resolved)
; incoming: call@0x0075ca->01ce:05ea
; incoming: call@0x00bde8->01ce:05ea
; incoming: call@0x00c02c->01ce:05ea
; byte_count: 55
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: 0x002693
; indirect_calls: 0
; routine_bytes_sha256: 18ac4b82ad55c5b35142cc8f7cf215d93681ee7e621839a39f3a85811369bc99

0028CA:  50                           push     ax
0028CB:  52                           push     dx
0028CC:  53                           push     bx
0028CD:  51                           push     cx
0028CE:  06                           push     es
0028CF:  56                           push     si
0028D0:  8B D6                        mov      dx, si
0028D2:  0E                           push     cs
0028D3:  E8 BD FD                     call     0x2693
0028D6:  66 65 8B 2E 8E 0A            mov      ebp, dword ptr gs:[0xa8e]
0028DC:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
0028E2:  75 16                        jne      0x28fa
0028E4:  B8 00 2F                     mov      ax, 0x2f00
0028E7:  CD 21                        int      0x21
0028E9:  8B F3                        mov      si, bx
0028EB:  83 C6 1A                     add      si, 0x1a
0028EE:  B9 18 00                     mov      cx, 0x18
0028F1:  B8 00 4E                     mov      ax, 0x4e00
0028F4:  CD 21                        int      0x21
0028F6:  66 26 8B 2C                  mov      ebp, dword ptr es:[si]
0028FA:  5E                           pop      si
0028FB:  07                           pop      es
0028FC:  59                           pop      cx
0028FD:  5B                           pop      bx
0028FE:  5A                           pop      dx
0028FF:  58                           pop      ax
002900:  CB                           retf    
