; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000c1a
; seg_off: 0000:061a
; group: seg_0000
; provenance: installed_interrupt_vector, manual_binary_boundary
; label: bloodprg_critical_error_handler
; label_comment: INT 24h handler installed by 0x000BFF. Records the incoming DOS critical-error DI code plus one in game data, enables interrupts, and returns the incoming action byte unchanged.
; incoming: int21_setvect_24@0x000c13->0000:061a
; byte_count: 12
; boundary: cfg_blocks_1_terminals_1
; terminal: iret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ce11d432425e014ba7088f22fcc1f3d3d57264a14453de2555933248451ca2eb

000C1A:  65 89 3E 9C 0A               mov      word ptr gs:[0xa9c], di
000C1F:  65 FF 06 9C 0A               inc      word ptr gs:[0xa9c]
000C24:  FB                           sti
000C25:  CF                           iret
