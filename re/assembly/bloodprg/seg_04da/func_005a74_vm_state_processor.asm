; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005a74
; seg_off: 04da:06d4
; group: seg_04da
; provenance: recursive_graph
; label: vm_state_processor
; label_comment: per-run VM state processor (called by vm_run_wrapper 0x55f5): lds si,gs:[0x6724] (state table), les di,gs:[0x672c]+0x10 (lookup); walks the object/line-record state before the exec loop. Prepares/updates the VM state each run
; byte_count: 137
; boundary: cfg_blocks_13_terminals_1
; terminal: ret:1
; direct_callees: 0x0061a6
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_005a74_vm_state_processor.cpp
; routine_bytes_sha256: b6c57749157210edff8c1aa442c08e7792fe972b3735833e502f121faf1096d0

005A74:  66 50                        push     eax
005A76:  53                           push     bx
005A77:  52                           push     dx
005A78:  1E                           push     ds
005A79:  06                           push     es
005A7A:  57                           push     di
005A7B:  56                           push     si
005A7C:  65 C5 36 24 67               lds      si, ptr gs:[0x6724]
005A81:  65 C4 3E 2C 67               les      di, ptr gs:[0x672c]
005A86:  83 C7 10                     add      di, 0x10
005A89:  66 33 C0                     xor      eax, eax
005A8C:  26 8B 35                     mov      si, word ptr es:[di]
005A8F:  83 3C 02                     cmp      word ptr [si], 2
005A92:  75 56                        jne      0x5aea
005A94:  8B 54 02                     mov      dx, word ptr [si + 2]
005A97:  65 F6 06 AA 67 03            test     byte ptr gs:[0x67aa], 3
005A9D:  75 1A                        jne      0x5ab9
005A9F:  65 F6 06 64 5E 01            test     byte ptr gs:[0x5e64], 1
005AA5:  74 0E                        je       0x5ab5
005AA7:  65 3B 36 54 67               cmp      si, word ptr gs:[0x6754]
005AAC:  75 0B                        jne      0x5ab9
005AAE:  65 3B 36 98 67               cmp      si, word ptr gs:[0x6798]
005AB3:  75 04                        jne      0x5ab9
005AB5:  81 E2 EF 7F                  and      dx, 0x7fef
005AB9:  E8 EA 06                     call     0x61a6
005ABC:  8B D8                        mov      bx, ax
005ABE:  56                           push     si
005ABF:  65 8B 36 50 67               mov      si, word ptr gs:[0x6750]
005AC4:  E8 DF 06                     call     0x61a6
005AC7:  8B F0                        mov      si, ax
005AC9:  66 8B 07                     mov      eax, dword ptr [bx]
005ACC:  66 3B 04                     cmp      eax, dword ptr [si]
005ACF:  74 12                        je       0x5ae3
005AD1:  65 8B 36 52 67               mov      si, word ptr gs:[0x6752]
005AD6:  E8 CD 06                     call     0x61a6
005AD9:  8B F0                        mov      si, ax
005ADB:  66 8B 07                     mov      eax, dword ptr [bx]
005ADE:  66 3B 04                     cmp      eax, dword ptr [si]
005AE1:  75 03                        jne      0x5ae6
005AE3:  80 CA 10                     or       dl, 0x10
005AE6:  5E                           pop      si
005AE7:  89 54 02                     mov      word ptr [si + 2], dx
005AEA:  83 C7 14                     add      di, 0x14
005AED:  26 80 7D 02 01               cmp      byte ptr es:[di + 2], 1
005AF2:  74 98                        je       0x5a8c
005AF4:  5E                           pop      si
005AF5:  5F                           pop      di
005AF6:  07                           pop      es
005AF7:  1F                           pop      ds
005AF8:  5A                           pop      dx
005AF9:  5B                           pop      bx
005AFA:  66 58                        pop      eax
005AFC:  C3                           ret     
