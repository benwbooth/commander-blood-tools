; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a1f3
; seg_off: 0971:04e3
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_refill_with_rollover_latch
; label_comment: shared early-return tail of ems_resource_flush 0x00a1b4. Publishes resource flag bit 7 through DS:0x0dac while 0x00a2ab refills the queue, clears the latch, then restores the parent routine's saved frame and returns directly to its caller. Natural C callers must call the recovered helper and then return instead of reproducing this nonlocal unwind.
; byte_count: 25
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: 0x00a2ab
; indirect_calls: 0
; routine_bytes_sha256: ce4c66a90382dcf5f5d1ff6bcc40e3e2a06fe251f52e1ead67a2f5453be23004

00A1F3:  A0 76 0D                     mov      al, byte ptr [0xd76]
00A1F6:  24 80                        and      al, 0x80
00A1F8:  A2 AC 0D                     mov      byte ptr [0xdac], al
00A1FB:  E8 AD 00                     call     0xa2ab
00A1FE:  C6 06 AC 0D 00               mov      byte ptr [0xdac], 0
00A203:  5D                           pop      bp
00A204:  5A                           pop      dx
00A205:  59                           pop      cx
00A206:  5B                           pop      bx
00A207:  5F                           pop      di
00A208:  07                           pop      es
00A209:  5E                           pop      si
00A20A:  1F                           pop      ds
00A20B:  C3                           ret     
