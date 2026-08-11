; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000ccb
; seg_off: 0000:06cb
; group: seg_0000
; provenance: recursive_graph
; label: init_early
; label_comment: called from entry
; byte_count: 36
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0xced:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: e520dea595ea78c64e73982aa4dfd9a0f7e71510613ddda409d9cddb4682d321

000CCB:  9C                           pushf   
000CCC:  33 C0                        xor      ax, ax
000CCE:  50                           push     ax
000CCF:  9D                           popf    
000CD0:  9C                           pushf   
000CD1:  58                           pop      ax
000CD2:  25 00 F0                     and      ax, 0xf000
000CD5:  3D 00 F0                     cmp      ax, 0xf000
000CD8:  74 11                        je       0xceb
000CDA:  B8 00 70                     mov      ax, 0x7000
000CDD:  50                           push     ax
000CDE:  9D                           popf    
000CDF:  9C                           pushf   
000CE0:  58                           pop      ax
000CE1:  25 00 70                     and      ax, 0x7000
000CE4:  74 05                        je       0xceb
000CE6:  B8 01 00                     mov      ax, 1
000CE9:  EB 02                        jmp      0xced
000CEB:  33 C0                        xor      ax, ax
000CED:  9D                           popf    
000CEE:  CB                           retf    
