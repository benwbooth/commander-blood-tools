; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000d0e
; seg_off: 0000:070e
; group: seg_0000
; provenance: relocation_proven_far_transfer_target
; label: poll_mouse
; label_comment: Polls INT 33h function 3 into GS:0x0a2a/0x0a2c coordinates and GS:0x0a2e button state for mouse_button_edges_update 0x1fbc
; incoming: call@0x001020->0000:070e
; byte_count: 60
; boundary: cfg_blocks_4_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 285c0d1c7d58630dd6764b8ac8cf090ad50d226c16a28801183e60259b6fc717

000D0E:  50                           push     ax
000D0F:  53                           push     bx
000D10:  51                           push     cx
000D11:  52                           push     dx
000D12:  B8 03 00                     mov      ax, 3
000D15:  CD 33                        int      0x33
000D17:  65 89 0E 2A 0A               mov      word ptr gs:[0xa2a], cx
000D1C:  65 89 16 2C 0A               mov      word ptr gs:[0xa2c], dx
000D21:  65 89 1E 2E 0A               mov      word ptr gs:[0xa2e], bx
000D26:  65 3B 0E 38 0A               cmp      cx, word ptr gs:[0xa38]
000D2B:  75 07                        jne      0xd34
000D2D:  65 3B 16 3A 0A               cmp      dx, word ptr gs:[0xa3a]
000D32:  74 11                        je       0xd45
000D34:  65 89 0E 38 0A               mov      word ptr gs:[0xa38], cx
000D39:  65 89 16 3A 0A               mov      word ptr gs:[0xa3a], dx
000D3E:  65 C7 06 3B 0B 00 00         mov      word ptr gs:[0xb3b], 0
000D45:  5A                           pop      dx
000D46:  59                           pop      cx
000D47:  5B                           pop      bx
000D48:  58                           pop      ax
000D49:  CB                           retf    
