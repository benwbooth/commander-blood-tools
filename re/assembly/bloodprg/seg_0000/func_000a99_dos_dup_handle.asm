; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000a99
; seg_off: 0000:0499
; group: seg_0000
; provenance: recursive_graph
; label: dos_dup_handle
; label_comment: DOS file-handle dup (ah=0x45) of the handle at gs:[0xa64] if valid (!=-1). Duplicates a file handle
; byte_count: 153
; boundary: cfg_blocks_17_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 4
; cxx_source: re/borland/bloodprg/seg_0000/func_000a99_dos_dup_handle.cpp
; routine_bytes_sha256: c2088183669bcd39f1cc2b541d1629478140910fc144a7995df4a72ed9866da1

000A99:  50                           push     ax
000A9A:  52                           push     dx
000A9B:  65 83 3E 64 0A FF            cmp      word ptr gs:[0xa64], -1
000AA1:  74 09                        je       0xaac
000AA3:  B4 45                        mov      ah, 0x45
000AA5:  65 8B 16 64 0A               mov      dx, word ptr gs:[0xa64]
000AAA:  CD 67                        int      0x67
000AAC:  65 83 3E 58 0A FF            cmp      word ptr gs:[0xa58], -1
000AB2:  74 09                        je       0xabd
000AB4:  B4 45                        mov      ah, 0x45
000AB6:  65 8B 16 58 0A               mov      dx, word ptr gs:[0xa58]
000ABB:  CD 67                        int      0x67
000ABD:  65 83 3E 5C 0A FF            cmp      word ptr gs:[0xa5c], -1
000AC3:  74 09                        je       0xace
000AC5:  B4 45                        mov      ah, 0x45
000AC7:  65 8B 16 5C 0A               mov      dx, word ptr gs:[0xa5c]
000ACC:  CD 67                        int      0x67
000ACE:  65 83 3E 60 0A FF            cmp      word ptr gs:[0xa60], -1
000AD4:  74 09                        je       0xadf
000AD6:  B4 45                        mov      ah, 0x45
000AD8:  65 8B 16 60 0A               mov      dx, word ptr gs:[0xa60]
000ADD:  CD 67                        int      0x67
000ADF:  65 83 3E 62 0A FF            cmp      word ptr gs:[0xa62], -1
000AE5:  74 0C                        je       0xaf3
000AE7:  B4 0A                        mov      ah, 0xa
000AE9:  65 8B 16 62 0A               mov      dx, word ptr gs:[0xa62]
000AEE:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000AF3:  65 83 3E 56 0A FF            cmp      word ptr gs:[0xa56], -1
000AF9:  74 0C                        je       0xb07
000AFB:  B4 0A                        mov      ah, 0xa
000AFD:  65 8B 16 56 0A               mov      dx, word ptr gs:[0xa56]
000B02:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000B07:  65 83 3E 5A 0A FF            cmp      word ptr gs:[0xa5a], -1
000B0D:  74 0C                        je       0xb1b
000B0F:  B4 0A                        mov      ah, 0xa
000B11:  65 8B 16 5A 0A               mov      dx, word ptr gs:[0xa5a]
000B16:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000B1B:  65 83 3E 5E 0A FF            cmp      word ptr gs:[0xa5e], -1
000B21:  74 0C                        je       0xb2f
000B23:  B4 0A                        mov      ah, 0xa
000B25:  65 8B 16 5E 0A               mov      dx, word ptr gs:[0xa5e]
000B2A:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000B2F:  5A                           pop      dx
000B30:  58                           pop      ax
000B31:  CB                           retf    
