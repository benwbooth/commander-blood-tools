; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000bff
; seg_off: 0000:05ff
; group: seg_0000
; provenance: recursive_graph
; label: install_ctrl_break_handler
; label_comment: startup: ds=cs; int21h ax=0x2523 (set interrupt vector 0x23 = Ctrl-Break handler) to 0x619. Traps Ctrl-Break so the game cleans up on exit
; byte_count: 26
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0000/func_000bff_install_ctrl_break_handler.cpp
; routine_bytes_sha256: 0576dc05ccd853136e3c3e3c936e4588052e3a92f626c1f9b14ccf53621064c1

000BFF:  50                           push     ax
000C00:  52                           push     dx
000C01:  1E                           push     ds
000C02:  8C C8                        mov      ax, cs
000C04:  8E D8                        mov      ds, ax
000C06:  B8 23 25                     mov      ax, 0x2523
000C09:  BA 19 06                     mov      dx, 0x619
000C0C:  CD 21                        int      0x21
000C0E:  B0 24                        mov      al, 0x24
000C10:  BA 1A 06                     mov      dx, 0x61a
000C13:  CD 21                        int      0x21
000C15:  1F                           pop      ds
000C16:  5A                           pop      dx
000C17:  58                           pop      ax
000C18:  CB                           retf    
