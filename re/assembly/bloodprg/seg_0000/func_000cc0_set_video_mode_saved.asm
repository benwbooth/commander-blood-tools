; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000cc0
; seg_off: 0000:06c0
; group: seg_0000
; provenance: recursive_graph
; label: set_video_mode_saved
; label_comment: restore/set the video mode: al=gs:[0x5232] (the saved entry mode from get_video_mode); int 10h ah=0 (set video mode). Restores the original text mode on exit / sets the saved mode
; byte_count: 11
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 3e670617ad028d66a8ff21ac003aa8ab73f1fe2c274a746feadf04f6d96ab450

000CC0:  50                           push     ax
000CC1:  33 C0                        xor      ax, ax
000CC3:  65 A0 32 52                  mov      al, byte ptr gs:[0x5232]
000CC7:  CD 10                        int      0x10
000CC9:  58                           pop      ax
000CCA:  CB                           retf    
