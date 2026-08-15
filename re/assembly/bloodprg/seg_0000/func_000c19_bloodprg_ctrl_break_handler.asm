; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000c19
; seg_off: 0000:0619
; group: seg_0000
; provenance: installed_interrupt_vector, manual_binary_boundary
; label: bloodprg_ctrl_break_handler
; label_comment: INT 23h handler installed by 0x000BFF. Ignores DOS Ctrl-Break by returning from the interrupt unchanged.
; incoming: int21_setvect_23@0x000c0c->0000:0619
; byte_count: 1
; boundary: cfg_blocks_1_terminals_1
; terminal: iret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 7a4a4b50f5121ed5310ece45a7eeb7af5545af63ee2ae52add4f37788f075b1d

000C19:  CF                           iret
