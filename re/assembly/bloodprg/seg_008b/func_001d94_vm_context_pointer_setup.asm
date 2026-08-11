; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001d94
; seg_off: 008b:0ee4
; group: seg_008b
; provenance: recursive_graph
; label: vm_context_pointer_setup
; label_comment: VM context setup: les di,gs:[0xabc] (work buffer); lds si,gs:[0x671c] (COD script); lfs bp,gs:[0x672c] (lookup table). Establishes the far pointers the VM exec loop reads (COD/lookup/buffer) before a run
; byte_count: 68
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x1dae:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_008b/func_001d94_vm_context_pointer_setup.cpp
; routine_bytes_sha256: dcd48c0f477c455a519ca60894afa403d371b1bff858f5ec024dd5137794a1d8

001D94:  06                           push     es
001D95:  57                           push     di
001D96:  1E                           push     ds
001D97:  56                           push     si
001D98:  0F A0                        push     fs
001D9A:  51                           push     cx
001D9B:  55                           push     bp
001D9C:  65 C4 3E BC 0A               les      di, ptr gs:[0xabc]
001DA1:  65 C5 36 1C 67               lds      si, ptr gs:[0x671c]
001DA6:  65 0F B4 2E 2C 67            lfs      bp, ptr gs:[0x672c]
001DAC:  33 C9                        xor      cx, cx
001DAE:  64 83 7E 10 FF               cmp      word ptr fs:[bp + 0x10], -1
001DB3:  74 18                        je       0x1dcd
001DB5:  64 83 7E 12 02               cmp      word ptr fs:[bp + 0x12], 2
001DBA:  75 0C                        jne      0x1dc8
001DBC:  64 8B 46 10                  mov      ax, word ptr fs:[bp + 0x10]
001DC0:  AB                           stosw    word ptr es:[di], ax
001DC1:  8B F0                        mov      si, ax
001DC3:  AC                           lodsb    al, byte ptr [si]
001DC4:  AA                           stosb    byte ptr es:[di], al
001DC5:  83 C1 03                     add      cx, 3
001DC8:  83 C5 14                     add      bp, 0x14
001DCB:  EB E1                        jmp      0x1dae
001DCD:  8B C1                        mov      ax, cx
001DCF:  5D                           pop      bp
001DD0:  59                           pop      cx
001DD1:  0F A1                        pop      fs
001DD3:  5E                           pop      si
001DD4:  1F                           pop      ds
001DD5:  5F                           pop      di
001DD6:  07                           pop      es
001DD7:  C3                           ret     
