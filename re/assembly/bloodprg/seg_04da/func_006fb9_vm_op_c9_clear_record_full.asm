; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006fb9
; seg_off: 04da:1c19
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c9_clear_record_full
; label_comment: VM opcode C9 consumes an absolute record offset in the segment from GS:0x6724. It reads the old kind, clears kind, then reads the old related offset from the incremented destination before clearing related and value. If the old kind is C4, it reads the related record's kind at that absolute offset, calls vm_field_offset(0x13, kind), adds the signed result with 16-bit wrap, clears GS:0x252A, sets GS:0x2531 to 6, and clears the reciprocal three-word record. Eight direct DOS vectors verify both paths, exact store/read ordering, positive/negative/zero offsets, wrap and alias cases, helper ABI, segments, registers, and flags.
; incoming: vm_opcode_handlers:opcode_0xc9
; byte_count: 58
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x006023
; indirect_calls: 0
; routine_bytes_sha256: 267d54cfc0f8f7c1a93c9065b2ec15f840a287b7d3b626a25c569d621c5b2d04

006FB9:  57                           push     di
006FBA:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006FBF:  AD                           lodsw    ax, word ptr [si]
006FC0:  8B F8                        mov      di, ax
006FC2:  33 C0                        xor      ax, ax
006FC4:  26 8B 0D                     mov      cx, word ptr es:[di]
006FC7:  AB                           stosw    word ptr es:[di], ax
006FC8:  26 8B 1D                     mov      bx, word ptr es:[di]
006FCB:  AB                           stosw    word ptr es:[di], ax
006FCC:  AB                           stosw    word ptr es:[di], ax
006FCD:  81 F9 C4 00                  cmp      cx, 0xc4
006FD1:  75 1E                        jne      0x6ff1
006FD3:  53                           push     bx
006FD4:  26 8B 1F                     mov      bx, word ptr es:[bx]
006FD7:  B8 13 00                     mov      ax, 0x13
006FDA:  E8 46 F0                     call     0x6023
006FDD:  5F                           pop      di
006FDE:  03 F8                        add      di, ax
006FE0:  33 C0                        xor      ax, ax
006FE2:  65 C6 06 2A 25 00            mov      byte ptr gs:[0x252a], 0
006FE8:  65 C6 06 31 25 06            mov      byte ptr gs:[0x2531], 6
006FEE:  AB                           stosw    word ptr es:[di], ax
006FEF:  AB                           stosw    word ptr es:[di], ax
006FF0:  AB                           stosw    word ptr es:[di], ax
006FF1:  5F                           pop      di
006FF2:  C3                           ret     
