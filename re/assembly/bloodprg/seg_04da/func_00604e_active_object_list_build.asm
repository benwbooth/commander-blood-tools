; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00604e
; seg_off: 04da:0cae
; group: seg_04da
; provenance: recursive_graph
; label: active_object_list_build
; label_comment: ACTIVE-OBJECT candidate list (output DS:0x6A16, 0xFFFF-terminated): walks the 20-byte directory gs:0x672C while +0x12 == 1 (STOPS at the first entry that is not, it does not skip), takes each object offset [si+0x10] and keeps it when fs:[obj+2] & 2 -- the IN-PLAY bit the story sets. Consumers: 0x721A (the nav chart) and the 2nd caller. PORTED: vm.rs build_active_object_list || ALSO RECORDED as `table_672c_process`: processes the 20-byte lookup table (2 calls): es=gs; di=0x6a16; lds si,gs:[0x672c] (the vm_lookup_table_20b); copies/transforms entries into 0x6a16. Prepares the 0x672c lookup data || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; byte_count: 65
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x6068:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_00604e_active_object_list_build.cpp
; routine_bytes_sha256: aae35b57bd4ca311aeb97c339d55521a01d8682843b3ed96c686e542b2f0ba5a

00604E:  50                           push     ax
00604F:  53                           push     bx
006050:  1E                           push     ds
006051:  56                           push     si
006052:  06                           push     es
006053:  57                           push     di
006054:  0F A0                        push     fs
006056:  8C E8                        mov      ax, gs
006058:  8E C0                        mov      es, ax
00605A:  BF 16 6A                     mov      di, 0x6a16
00605D:  65 C5 36 2C 67               lds      si, ptr gs:[0x672c]
006062:  65 0F B4 1E 24 67            lfs      bx, ptr gs:[0x6724]
006068:  8B 44 12                     mov      ax, word ptr [si + 0x12]
00606B:  83 F8 01                     cmp      ax, 1
00606E:  75 12                        jne      0x6082
006070:  8B 5C 10                     mov      bx, word ptr [si + 0x10]
006073:  64 F6 47 02 02               test     byte ptr fs:[bx + 2], 2
006078:  74 03                        je       0x607d
00607A:  8B C3                        mov      ax, bx
00607C:  AB                           stosw    word ptr es:[di], ax
00607D:  83 C6 14                     add      si, 0x14
006080:  EB E6                        jmp      0x6068
006082:  B8 FF FF                     mov      ax, 0xffff
006085:  AB                           stosw    word ptr es:[di], ax
006086:  0F A1                        pop      fs
006088:  5F                           pop      di
006089:  07                           pop      es
00608A:  5E                           pop      si
00608B:  1F                           pop      ds
00608C:  5B                           pop      bx
00608D:  58                           pop      ax
00608E:  C3                           ret     
