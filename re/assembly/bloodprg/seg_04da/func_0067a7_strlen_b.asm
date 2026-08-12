; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0067a7
; seg_off: 061a:0007
; group: seg_04da
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: strlen_b
; label_comment: near bounded string-length helper; scans at most 0xffff bytes from ES:DI with REPNE SCASB, returns the byte length in AX when terminated or 0xfffe at the unterminated bound, and preserves CX/DI
; incoming: call@0x006701->0x0067a7
; byte_count: 19
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 09ec15c0297c7bc0f87067bf166a91ff5ad26351f57887f15a8366eff7c4cf10

0067A7:  51                           push     cx
0067A8:  57                           push     di
0067A9:  B9 FF FF                     mov      cx, 0xffff
0067AC:  32 C0                        xor      al, al
0067AE:  F2 AE                        repne scasb al, byte ptr es:[di]
0067B0:  F7 D9                        neg      cx
0067B2:  8B C1                        mov      ax, cx
0067B4:  83 E8 02                     sub      ax, 2
0067B7:  5F                           pop      di
0067B8:  59                           pop      cx
0067B9:  C3                           ret
