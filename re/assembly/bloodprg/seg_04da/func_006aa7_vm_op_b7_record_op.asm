; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006aa7
; seg_off: 04da:1707
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_b7_record_op
; label_comment: VM opcode 0xB7: les di,gs:[0x6724] (object/line-record state table); optional 0xA1(POP) skip; lodsw index/offset + lodsb value -> read/write a field in the state-table record. An object/line-record manipulation opcode || NARROWER EARLIER READING `vm_bit_set_test_6aa7`: 0xB7 high-bit-first byte flag set/clear/test handler || MERGED 2026-07-25 (audit-fixes #133): one address, two names, the shorter describing a prologue or a single facet. Kept because a narrow reading records a true observation; renamed away because it is not what the routine IS.
; incoming: vm_opcode_handlers:opcode_0xb7
; byte_count: 95
; boundary: cfg_blocks_12_terminals_4
; terminal: jmp 0x6b04:3, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 1e863e6a5453b52208bd8aca4aca35235b6d5df3f421fe125af59cefad1142d9

006AA7:  57                           push     di
006AA8:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006AAD:  32 D2                        xor      dl, dl
006AAF:  8A 04                        mov      al, byte ptr [si]
006AB1:  3C A1                        cmp      al, 0xa1
006AB3:  75 03                        jne      0x6ab8
006AB5:  FE C2                        inc      dl
006AB7:  46                           inc      si
006AB8:  AD                           lodsw    ax, word ptr [si]
006AB9:  8B D8                        mov      bx, ax
006ABB:  AC                           lodsb    al, byte ptr [si]
006ABC:  32 E4                        xor      ah, ah
006ABE:  8B C8                        mov      cx, ax
006AC0:  80 E1 07                     and      cl, 7
006AC3:  C1 E8 03                     shr      ax, 3
006AC6:  03 D8                        add      bx, ax
006AC8:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006ACE:  74 1B                        je       0x6aeb
006AD0:  26 8A 01                     mov      al, byte ptr es:[bx + di]
006AD3:  D2 E0                        shl      al, cl
006AD5:  D0 E0                        shl      al, 1
006AD7:  73 09                        jae      0x6ae2
006AD9:  0A D2                        or       dl, dl
006ADB:  74 27                        je       0x6b04
006ADD:  E8 82 F9                     call     0x6462
006AE0:  EB 22                        jmp      0x6b04
006AE2:  0A D2                        or       dl, dl
006AE4:  75 1E                        jne      0x6b04
006AE6:  E8 79 F9                     call     0x6462
006AE9:  EB 19                        jmp      0x6b04
006AEB:  F6 D9                        neg      cl
006AED:  FE C9                        dec      cl
006AEF:  80 E1 07                     and      cl, 7
006AF2:  B0 01                        mov      al, 1
006AF4:  D2 E0                        shl      al, cl
006AF6:  0A D2                        or       dl, dl
006AF8:  75 05                        jne      0x6aff
006AFA:  26 08 01                     or       byte ptr es:[bx + di], al
006AFD:  EB 05                        jmp      0x6b04
006AFF:  F6 D0                        not      al
006B01:  26 20 01                     and      byte ptr es:[bx + di], al
006B04:  5F                           pop      di
006B05:  C3                           ret     
