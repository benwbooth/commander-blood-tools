; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006023
; seg_off: 04da:0c83
; group: seg_04da
; provenance: recursive_graph
; label: vm_field_offset
; label_comment: FIELD-OFFSET RESOLVER, fully decoded: SHL AX,4 (selector*16) then BSF BX,BX -- BIT SCAN FORWARD on the kind -- then ADD BX,AX and MOV AL,gs:[bx+0x6D60]. So the KIND IS A BITMASK and the matrix column is the index of its LOWEST SET BIT, not the kind value: column k corresponds to kind value 2^k (column 8 = kind 0x100). vm.rs vm_field_offset models this exactly with kind.trailing_zeros(), and its `if kind == 0 { None }` guard matches BSF leaving the destination undefined for a zero source || MERGED 2026-07-25 (audit-fixes #130), also recorded as: helper: returns GS:0x6D60[selector*16 + bsf(kind)] as a kind-specific record-field offset
; byte_count: 17
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 7dbb54cd24e4f0a70c96b557a988933bd79ff057be6881d5817d38fdb14ea932

006023:  53                           push     bx
006024:  C1 E0 04                     shl      ax, 4
006027:  0F BC DB                     bsf      bx, bx
00602A:  03 D8                        add      bx, ax
00602C:  65 8A 87 60 6D               mov      al, byte ptr gs:[bx + 0x6d60]
006031:  98                           cwde    
006032:  5B                           pop      bx
006033:  C3                           ret     
