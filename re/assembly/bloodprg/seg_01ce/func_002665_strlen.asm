; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002665
; seg_off: 01ce:0385
; group: seg_01ce
; provenance: relocation_proven_far_transfer_target
; label: strlen
; label_comment: null-terminated string length: cx=0xffff; repne scasb (scan es:di for 0); neg cx -> length. Standard C strlen helper
; incoming: call@0x000d99->01ce:0385
; incoming: call@0x000dc2->01ce:0385
; incoming: call@0x000df2->01ce:0385
; incoming: call@0x000e15->01ce:0385
; byte_count: 19
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 6e2373a08942d4b119e6d1336777d439d7ffb47d63f83afd33cf4113382e2718

002665:  51                           push     cx
002666:  57                           push     di
002667:  B9 FF FF                     mov      cx, 0xffff
00266A:  33 C0                        xor      ax, ax
00266C:  F2 AE                        repne scasb al, byte ptr es:[di]
00266E:  F7 D9                        neg      cx
002670:  8B C1                        mov      ax, cx
002672:  83 E8 02                     sub      ax, 2
002675:  5F                           pop      di
002676:  59                           pop      cx
002677:  CB                           retf    
