; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0071cf
; seg_off: 04da:1e2f
; group: seg_04da
; provenance: relocation_proven_far_transfer_target
; label: vm_lookup_prep
; label_comment: VM lookup prep (also 0x721a): call table_672c_process 0x604e; si=0x6a16 (the processed lookup output). Prepares the 0x672c->0x6a16 lookup for VM record resolution
; incoming: call@0x00873c->04da:1e2f
; byte_count: 75
; boundary: cfg_blocks_8_terminals_2
; terminal: jmp 0x71e6:1, retf:1
; direct_callees: 0x00604e
; indirect_calls: 0
; routine_bytes_sha256: aaf60d2d580ad987ffdbb75dff624f3369cf2c58fc25a5b1abf4a632fba4d1b2

0071CF:  53                           push     bx
0071D0:  06                           push     es
0071D1:  57                           push     di
0071D2:  56                           push     si
0071D3:  55                           push     bp
0071D4:  66 33 C0                     xor      eax, eax
0071D7:  8B C8                        mov      cx, ax
0071D9:  E8 72 EE                     call     0x604e
0071DC:  BE 16 6A                     mov      si, 0x6a16
0071DF:  C4 3E 24 67                  les      di, ptr [0x6724]
0071E3:  BD 13 2B                     mov      bp, 0x2b13
0071E6:  AD                           lodsw    ax, word ptr [si]
0071E7:  83 F8 FF                     cmp      ax, -1
0071EA:  74 21                        je       0x720d
0071EC:  65 3B 06 54 67               cmp      ax, word ptr gs:[0x6754]
0071F1:  74 18                        je       0x720b
0071F3:  65 3B 06 56 67               cmp      ax, word ptr gs:[0x6756]
0071F8:  74 11                        je       0x720b
0071FA:  67 26 8B 1C 38               mov      bx, word ptr es:[eax + edi]
0071FF:  83 FB 02                     cmp      bx, 2
007202:  75 07                        jne      0x720b
007204:  89 46 00                     mov      word ptr [bp], ax
007207:  83 C5 02                     add      bp, 2
00720A:  41                           inc      cx
00720B:  EB D9                        jmp      0x71e6
00720D:  C7 46 00 FF FF               mov      word ptr [bp], 0xffff
007212:  8B C1                        mov      ax, cx
007214:  5D                           pop      bp
007215:  5E                           pop      si
007216:  5F                           pop      di
007217:  07                           pop      es
007218:  5B                           pop      bx
007219:  CB                           retf    
