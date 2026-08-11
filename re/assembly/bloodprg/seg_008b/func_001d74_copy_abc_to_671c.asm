; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001d74
; seg_off: 008b:0ec4
; group: seg_008b
; provenance: recursive_graph
; label: copy_abc_to_671c
; label_comment: buffer copy: lds si,gs:[0xabc]; les di,gs:[0x671c]; lodsw loop. Word-copies from the 0xabc source buffer into the 0x671c object/work area
; byte_count: 32
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_008b/func_001d74_copy_abc_to_671c.cpp
; routine_bytes_sha256: 30c8f13284ab8cb77ad2283714851fe93e0dbeecc377800db9c5488a42f8da2c

001D74:  1E                           push     ds
001D75:  56                           push     si
001D76:  06                           push     es
001D77:  57                           push     di
001D78:  51                           push     cx
001D79:  8B C8                        mov      cx, ax
001D7B:  65 C5 36 BC 0A               lds      si, ptr gs:[0xabc]
001D80:  65 C4 3E 1C 67               les      di, ptr gs:[0x671c]
001D85:  AD                           lodsw    ax, word ptr [si]
001D86:  8B F8                        mov      di, ax
001D88:  A4                           movsb    byte ptr es:[di], byte ptr [si]
001D89:  83 E9 03                     sub      cx, 3
001D8C:  75 F7                        jne      0x1d85
001D8E:  59                           pop      cx
001D8F:  5F                           pop      di
001D90:  07                           pop      es
001D91:  5E                           pop      si
001D92:  1F                           pop      ds
001D93:  C3                           ret     
