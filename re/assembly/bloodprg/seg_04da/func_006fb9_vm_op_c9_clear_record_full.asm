; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006fb9
; seg_off: 04da:1c19
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c9_clear_record_full
; label_comment: 0xC9 FULL decode: lodsw record offset; cx=old type at +0; ZEROES THE WHOLE 3-WORD RECORD (stosw x3 @0x6FC7/0x6FCB/0x6FCC) - not just +0; bx = the word at +2 (related record) read BEFORE clearing. If cx==0xC4: bx=kind of related, ax=vm_field_offset(0x13,kind), di=related+ax, then gs:[0x252A]=0 + gs:[0x2531]=6 and three more zero words clearing the related object's selector-0x13 RECIPROCAL triple. Leaving that stale wedges the actor: the C4 mode-0 write guard 0x6CE9..0x6CFF refuses a new presentation while the related selector-0x13 field still reads 0xC4. Ported: live step() 0xC9 arm (src/vm.rs); the 0x252A/0x2531 writes still pending the ship-3D nav setter || ALSO RECORDED as `vm_op_c9_clear_record`: 0xC9 CLEAR RECORD. les di,gs:[0x6724]; lodsw -> record offset; XOR AX,AX; reads the OLD type (es:[di]) and OLD related (es:[di+2]) BEFORE overwriting, then THREE STOSW zero +0/+2/+4 -- this is what fixes the 3-word record layout. If the old type was 0xC4 (CMP CX,0xC4 @0x6FCD) it runs the reciprocal teardown: follow the related pointer, offset it by vm_field_offset(selector 0x13) via call 0x6023, and clear gs:0x252A. Ported as vm.rs clear_record / clear_record_words || ALSO RECORDED as `vm_op_c9_clear_record_field`: VM opcode 0xC9: les di,gs:[0x6724]; lodsw di = record offset; read cx=es:[di]; stosw 0 - clears a field in the object/line-record state table to 0. A reset-record-field op || ALSO RECORDED as `vm_op_c9_record_clear`: 0xC9 record-clear handler; zeros es:[record..record+4] and clears related 0xc4 actor subrecord || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xc9
; byte_count: 58
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x006023
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_006fb9_vm_op_c9_clear_record_full.cpp
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
